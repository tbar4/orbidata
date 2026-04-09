use crate::config::Config;
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

        Self {
            config: Arc::new(config),
            http_client,
            tle_cache,
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
