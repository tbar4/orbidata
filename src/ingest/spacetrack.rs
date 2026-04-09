use anyhow::{anyhow, Context};
use chrono::{DateTime, Duration, Utc};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::models::orbital_element::CelesTrakGp;

const BASE_URL: &str = "https://www.space-track.org";
const CDM_QUERY: &str =
    "/basicspacedata/query/class/cdm_public/orderby/TCA%20asc/limit/20/format/json";

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

/// Raw record from Space-Track `tle` class — all numeric fields are strings.
/// Used for historical TLE epoch queries.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct SpaceTrackTleRecord {
    #[serde(rename = "OBJECT_NAME")]
    pub object_name: Option<String>,
    #[serde(rename = "OBJECT_ID")]
    pub object_id: Option<String>,
    #[serde(rename = "NORAD_CAT_ID")]
    pub norad_cat_id: Option<String>,
    #[serde(rename = "OBJECT_TYPE")]
    pub object_type: Option<String>,
    #[serde(rename = "EPOCH")]
    pub epoch: Option<String>,
    #[serde(rename = "MEAN_MOTION")]
    pub mean_motion: Option<String>,
    #[serde(rename = "ECCENTRICITY")]
    pub eccentricity: Option<String>,
    #[serde(rename = "INCLINATION")]
    pub inclination: Option<String>,
    #[serde(rename = "RA_OF_ASC_NODE")]
    pub ra_of_asc_node: Option<String>,
    #[serde(rename = "ARG_OF_PERICENTER")]
    pub arg_of_pericenter: Option<String>,
    #[serde(rename = "MEAN_ANOMALY")]
    pub mean_anomaly: Option<String>,
    #[serde(rename = "BSTAR")]
    pub bstar: Option<String>,
    #[serde(rename = "MEAN_MOTION_DOT")]
    pub mean_motion_dot: Option<String>,
    #[serde(rename = "MEAN_MOTION_DDOT")]
    pub mean_motion_ddot: Option<String>,
    #[serde(rename = "PERIOD")]
    pub period: Option<String>,
    #[serde(rename = "APOAPSIS")]
    pub apoapsis: Option<String>,
    #[serde(rename = "PERIAPSIS")]
    pub periapsis: Option<String>,
    #[serde(rename = "COUNTRY_CODE")]
    pub country_code: Option<String>,
    #[serde(rename = "LAUNCH_DATE")]
    pub launch_date: Option<String>,
    #[serde(rename = "SITE")]
    pub site: Option<String>,
    #[serde(rename = "RCS_SIZE")]
    pub rcs_size: Option<String>,
    #[serde(rename = "TLE_LINE1")]
    pub tle_line1: Option<String>,
    #[serde(rename = "TLE_LINE2")]
    pub tle_line2: Option<String>,
}

fn parse_f64(s: Option<String>) -> f64 {
    s.and_then(|v| v.trim().parse::<f64>().ok()).unwrap_or(0.0)
}

fn parse_opt_f64(s: Option<String>) -> Option<f64> {
    s.and_then(|v| v.trim().parse::<f64>().ok())
}

fn parse_u32(s: Option<String>) -> u32 {
    s.and_then(|v| v.trim().parse::<u32>().ok()).unwrap_or(0)
}

impl From<SpaceTrackTleRecord> for crate::models::orbital_element::OrbitalElement {
    fn from(r: SpaceTrackTleRecord) -> Self {
        use crate::models::orbital_element::{
            KeplerianElements, OrbitalElement, SatelliteMetadata, TleLines,
        };

        let tle = match (r.tle_line1, r.tle_line2) {
            (Some(l1), Some(l2)) => Some(TleLines {
                line1: l1,
                line2: l2,
            }),
            _ => None,
        };

        OrbitalElement {
            norad_id: parse_u32(r.norad_cat_id),
            name: r.object_name.unwrap_or_default(),
            object_id: r.object_id,
            object_type: r.object_type,
            epoch: r.epoch.unwrap_or_default(),
            elements: KeplerianElements {
                mean_motion_rev_per_day: parse_f64(r.mean_motion),
                eccentricity: parse_f64(r.eccentricity),
                inclination_deg: parse_f64(r.inclination),
                raan_deg: parse_f64(r.ra_of_asc_node),
                arg_of_pericenter_deg: parse_f64(r.arg_of_pericenter),
                mean_anomaly_deg: parse_f64(r.mean_anomaly),
                bstar: parse_f64(r.bstar),
                semimajor_axis_km: None,
                period_min: parse_opt_f64(r.period),
                apoapsis_km: parse_opt_f64(r.apoapsis),
                periapsis_km: parse_opt_f64(r.periapsis),
            },
            tle,
            metadata: SatelliteMetadata {
                country_code: r.country_code,
                launch_date: r.launch_date,
                decay_date: None,
                rcs_size: r.rcs_size,
                site: r.site,
            },
        }
    }
}

pub struct SpaceTrackClient {
    pub http: Client,
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub session: Arc<RwLock<SpaceTrackSession>>,
}

impl SpaceTrackClient {
    pub fn new(username: String, password: String) -> Result<Self, anyhow::Error> {
        Self::with_base_url(BASE_URL.to_string(), username, password)
    }

    pub fn with_base_url(
        base_url: String,
        username: String,
        password: String,
    ) -> Result<Self, anyhow::Error> {
        let http = Client::builder()
            .use_rustls_tls()
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build Space-Track HTTP client")?;

        Ok(Self {
            http,
            base_url,
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
            .post(format!("{}/ajaxauth/login", self.base_url))
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

        let url = format!("{}{}", self.base_url, CDM_QUERY);
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

    /// Fetch historical OMM epochs from the Space-Track `gp_history` class for a given NORAD ID.
    /// If start/end dates are provided (YYYY-MM-DD), filters to that range.
    /// Otherwise returns the most recent `limit` epochs ordered newest-first.
    pub async fn fetch_tle_history(
        &self,
        norad_id: u32,
        limit: u32,
        start: Option<&str>,
        end: Option<&str>,
    ) -> Result<Vec<CelesTrakGp>, anyhow::Error> {
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

        // Record request
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

        let url = match (start, end) {
            (Some(s), Some(e)) => format!(
                "{}/basicspacedata/query/class/gp_history/NORAD_CAT_ID/{}/EPOCH/{}--{}/orderby/EPOCH%20asc/limit/{}/format/json",
                self.base_url, norad_id, s, e, limit
            ),
            _ => format!(
                "{}/basicspacedata/query/class/gp_history/NORAD_CAT_ID/{}/orderby/EPOCH%20desc/limit/{}/format/json",
                self.base_url, norad_id, limit
            ),
        };

        info!(url = %url, norad_id, limit, "Requesting Space-Track gp_history");

        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .context("Space-Track gp_history request failed")?;

        let status = resp.status();
        info!(status = %status, norad_id, "Space-Track gp_history response status");

        match status {
            StatusCode::OK => {
                let body = resp
                    .text()
                    .await
                    .context("Failed to read Space-Track gp_history response body")?;

                tracing::debug!(chars = body.len(), "Space-Track gp_history raw response");

                let records: Vec<CelesTrakGp> =
                    serde_json::from_str(&body).map_err(|e| {
                        let preview = if body.len() > 300 {
                            &body[..300]
                        } else {
                            &body
                        };
                        tracing::error!(
                            norad_id,
                            "Space-Track gp_history returned unexpected response (not a JSON array). \
                             Parse error: {}. Response preview: {}",
                            e,
                            preview
                        );
                        anyhow::anyhow!(
                            "Space-Track gp_history response is not a JSON array: {}. Preview: {}",
                            e,
                            preview
                        )
                    })?;
                info!(
                    norad_id,
                    count = records.len(),
                    "Space-Track gp_history fetch complete"
                );
                Ok(records)
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let mut session = self.session.write().await;
                session.rate_limit.set_backoff();
                Err(anyhow!(
                    "Space-Track rate limit exceeded (429) — backing off 60 seconds"
                ))
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                {
                    let mut session = self.session.write().await;
                    session.authenticated = false;
                }
                Err(anyhow!(
                    "Space-Track session expired — re-authentication required"
                ))
            }
            other => {
                let body = resp.text().await.unwrap_or_default();
                let body_preview = &body[..body.len().min(500)];
                warn!(status = %other, body_preview = %body_preview, "Space-Track gp_history returned error");
                Err(anyhow!(
                    "Space-Track gp_history returned unexpected status: {}",
                    other
                ))
            }
        }
    }
}
