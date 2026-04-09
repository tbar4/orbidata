use anyhow::{anyhow, Context};
use chrono::{DateTime, Duration, Utc};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

const BASE_URL: &str = "https://www.space-track.org";
const CDM_QUERY: &str =
    "/basicspacedata/query/class/cdm_public/orderby/TCA asc/limit/20/format/json";

/// Tracks rate-limit state. Space-Track allows 30 req/min per account.
#[derive(Debug, Default)]
pub struct RateLimitState {
    pub request_count: u32,
    pub window_start: Option<DateTime<Utc>>,
    pub backoff_until: Option<DateTime<Utc>>,
}

impl RateLimitState {
    /// Returns true if we are currently in a backoff period.
    pub fn is_backing_off(&self) -> bool {
        self.backoff_until.map(|t| Utc::now() < t).unwrap_or(false)
    }

    /// Returns remaining backoff seconds, or 0 if not in backoff.
    pub fn backoff_secs_remaining(&self) -> i64 {
        self.backoff_until
            .map(|t| (t - Utc::now()).num_seconds().max(0))
            .unwrap_or(0)
    }

    /// Record a request and reset window if > 60s has passed.
    pub fn record_request(&mut self) {
        let now = Utc::now();
        match self.window_start {
            None => {
                self.window_start = Some(now);
                self.request_count = 1;
            }
            Some(start) if (now - start).num_seconds() >= 60 => {
                self.window_start = Some(now);
                self.request_count = 1;
            }
            _ => {
                self.request_count += 1;
            }
        }
    }

    /// Returns true if we've hit 28 requests in the current window (2 req margin).
    pub fn is_near_limit(&self) -> bool {
        self.request_count >= 28
    }

    /// Set a backoff until +60s from now.
    pub fn set_backoff(&mut self) {
        let backoff_until = Utc::now() + Duration::seconds(60);
        self.backoff_until = Some(backoff_until);
        warn!(
            "Space-Track rate limit hit — backing off until {}",
            backoff_until
        );
    }
}

/// Shared Space-Track session state.
#[derive(Debug, Default)]
pub struct SpaceTrackSession {
    pub authenticated: bool,
    pub authenticated_at: Option<DateTime<Utc>>,
    pub rate_limit: RateLimitState,
}

impl SpaceTrackSession {
    /// Returns true if authenticated and session is less than 90 minutes old.
    pub fn is_session_valid(&self) -> bool {
        if !self.authenticated {
            return false;
        }
        self.authenticated_at
            .map(|t| (Utc::now() - t).num_minutes() < 90)
            .unwrap_or(false)
    }
}

/// Raw CDM record as returned by Space-Track (all fields are strings).
#[derive(Debug, Deserialize, Clone)]
pub struct SpaceTrackCdm {
    #[serde(rename = "CDM_ID")]
    pub cdm_id: String,
    #[serde(rename = "CREATED")]
    pub created: Option<String>,
    #[serde(rename = "EMERGENCY_REPORTABLE")]
    pub emergency_reportable: Option<String>,
    #[serde(rename = "TCA")]
    pub tca: Option<String>,
    #[serde(rename = "MIN_RNG")]
    pub min_rng: Option<String>,
    #[serde(rename = "PC")]
    pub pc: Option<String>,
    #[serde(rename = "SAT_1_ID")]
    pub sat_1_id: Option<String>,
    #[serde(rename = "SAT_1_NAME")]
    pub sat_1_name: Option<String>,
    #[serde(rename = "SAT_2_ID")]
    pub sat_2_id: Option<String>,
    #[serde(rename = "SAT_2_NAME")]
    pub sat_2_name: Option<String>,
    #[serde(rename = "SAT_1_OBJECT_TYPE")]
    pub sat_1_object_type: Option<String>,
    #[serde(rename = "SAT_2_OBJECT_TYPE")]
    pub sat_2_object_type: Option<String>,
    #[serde(rename = "COLLISION_PERCENTILE")]
    pub collision_percentile: Option<String>,
}

pub struct SpaceTrackClient {
    pub http: Client,
    pub username: String,
    pub password: String,
    pub session: Arc<RwLock<SpaceTrackSession>>,
}

impl SpaceTrackClient {
    pub fn new(username: String, password: String) -> Result<Self, anyhow::Error> {
        let http = Client::builder()
            .use_rustls_tls()
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build Space-Track HTTP client")?;

        Ok(Self {
            http,
            username,
            password,
            session: Arc::new(RwLock::new(SpaceTrackSession::default())),
        })
    }

    /// Authenticate with Space-Track.
    pub async fn authenticate(&self) -> Result<(), anyhow::Error> {
        info!("Authenticating with Space-Track");
        let params = [
            ("identity", self.username.as_str()),
            ("password", self.password.as_str()),
        ];
        let resp = self
            .http
            .post(format!("{}/ajaxauth/login", BASE_URL))
            .form(&params)
            .send()
            .await
            .context("Space-Track login request failed")?;

        let status = resp.status();

        if status == StatusCode::OK {
            let body = resp.text().await.unwrap_or_default();
            if body.contains("\"Failed\"") {
                return Err(anyhow!(
                    "Space-Track authentication failed: invalid credentials"
                ));
            }
            let mut session = self.session.write().await;
            session.authenticated = true;
            session.authenticated_at = Some(Utc::now());
            info!("Space-Track authentication successful");
            Ok(())
        } else {
            Err(anyhow!(
                "Space-Track login returned unexpected status: {}",
                status
            ))
        }
    }

    /// Fetch CDMs. Handles session validation, re-auth, rate limiting, and 429 backoff.
    pub async fn fetch_cdms(&self) -> Result<Vec<SpaceTrackCdm>, anyhow::Error> {
        // Check backoff
        {
            let session = self.session.read().await;
            if session.rate_limit.is_backing_off() {
                let secs = session.rate_limit.backoff_secs_remaining();
                return Err(anyhow!(
                    "Rate limit backoff active — retry in {} seconds",
                    secs
                ));
            }
        }

        // Authenticate if needed
        {
            let needs_auth = {
                let session = self.session.read().await;
                !session.is_session_valid()
            };
            if needs_auth {
                self.authenticate().await?;
            }
        }

        // Record request and check near-limit
        {
            let mut session = self.session.write().await;
            session.rate_limit.record_request();
            if session.rate_limit.is_near_limit() {
                warn!(
                    "Approaching Space-Track rate limit ({} req in window)",
                    session.rate_limit.request_count
                );
            }
        }

        let url = format!("{}{}", BASE_URL, CDM_QUERY);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .context("Space-Track CDM query failed")?;

        match resp.status() {
            StatusCode::OK => {
                let cdms: Vec<SpaceTrackCdm> = resp
                    .json()
                    .await
                    .context("Failed to deserialize Space-Track CDM response")?;
                info!("Fetched {} CDM records from Space-Track", cdms.len());
                Ok(cdms)
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let mut session = self.session.write().await;
                session.rate_limit.set_backoff();
                Err(anyhow!(
                    "Space-Track rate limit exceeded (429) — backing off 60 seconds"
                ))
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                let mut session = self.session.write().await;
                session.authenticated = false;
                Err(anyhow!(
                    "Space-Track session expired — re-authentication required"
                ))
            }
            status => Err(anyhow!("Space-Track CDM query returned status: {}", status)),
        }
    }
}
