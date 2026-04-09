use crate::config::Config;
use crate::ingest::spacetrack::SpaceTrackClient;
use crate::models::orbital_element::OrbitalElement;
use moka::future::Cache;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

const TLE_CACHE_KEY: &str = "active_satellites";

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub http_client: Client,
    pub tle_cache: Cache<String, Arc<Vec<OrbitalElement>>>,
    pub spacetrack: Option<Arc<SpaceTrackClient>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let http_client = Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        let tle_cache = Cache::builder()
            .max_capacity(10)
            .time_to_live(Duration::from_secs(config.tle_cache_ttl_secs))
            .build();

        let spacetrack =
            if config.spacetrack_username.is_some() && config.spacetrack_password.is_some() {
                let username = config.spacetrack_username.clone().unwrap();
                let password = config.spacetrack_password.clone().unwrap();
                match SpaceTrackClient::new(username, password) {
                    Ok(client) => {
                        tracing::info!("Space-Track client initialized");
                        Some(Arc::new(client))
                    }
                    Err(e) => {
                        tracing::warn!("Failed to initialize Space-Track client: {}", e);
                        None
                    }
                }
            } else {
                tracing::info!(
                    "No Space-Track credentials configured — /v1/conjunctions/live will return 503"
                );
                None
            };

        Self {
            config: Arc::new(config),
            http_client,
            tle_cache,
            spacetrack,
        }
    }

    pub async fn get_cached_tles(&self) -> Option<Arc<Vec<OrbitalElement>>> {
        self.tle_cache.get(TLE_CACHE_KEY).await
    }

    pub async fn set_cached_tles(&self, tles: Vec<OrbitalElement>) {
        self.tle_cache
            .insert(TLE_CACHE_KEY.to_string(), Arc::new(tles))
            .await;
    }
}
