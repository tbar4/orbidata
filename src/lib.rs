pub mod api;
pub mod config;
pub mod error;
pub mod ingest;
pub mod models;
pub mod state;

use axum::{routing::get, Router};
use state::AppState;

pub fn build_app(state: AppState) -> Router {
    let tle_router = Router::new()
        .route("/", get(api::tle::list_tles))
        .route("/:norad_id", get(api::tle::get_tle))
        .route("/:norad_id/history", get(api::tle::get_tle_history));

    Router::new()
        .route("/v1/health", get(api::health::health))
        .nest("/v1/tle", tle_router)
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
