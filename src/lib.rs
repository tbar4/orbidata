pub mod api;
pub mod config;
pub mod error;
pub mod ingest;
pub mod models;
pub mod state;

use axum::{routing::get, Router};
use state::AppState;

pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(api::health::health))
        .route("/v1/tle", get(api::tle::list_tles))
        .route("/v1/tle/:norad_id/history", get(api::tle::get_tle_history))
        .route("/v1/tle/:norad_id", get(api::tle::get_tle))
        .route(
            "/v1/conjunctions",
            get(api::conjunctions::list_conjunctions),
        )
        .route(
            "/v1/conjunctions/live",
            get(api::conjunctions::list_conjunctions_live),
        )
        .with_state(state)
}
