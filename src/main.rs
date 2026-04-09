use axum::{routing::get, Router};
use clap::Parser;
use std::net::SocketAddr;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod api;
mod config;
mod error;
mod ingest;
mod models;
mod state;

use config::Config;
use state::AppState;

async fn fallback_404(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
    tracing::warn!("No route matched: {}", uri);
    (
        axum::http::StatusCode::NOT_FOUND,
        axum::Json(serde_json::json!({
            "error": {
                "code": 404,
                "message": format!("No route found for {}", uri)
            }
        })),
    )
}

#[tokio::main]
async fn main() {
    // Load .env file if present (silently ignored if absent)
    dotenvy::dotenv().ok();

    let config = Config::parse();

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState::new(config.clone());

    if state.spacetrack.is_some() {
        tracing::info!("Space-Track integration: ENABLED (live CDM + TLE history active)");
    } else {
        tracing::warn!(
            "Space-Track integration: DISABLED — set SPACETRACK_USERNAME and SPACETRACK_PASSWORD in .env to enable /v1/conjunctions/live and /v1/tle/{{id}}/history"
        );
    }

    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("Invalid bind address");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Nest TLE routes under a sub-router to avoid matchit parameter conflicts
    // between /{norad_id} and /{norad_id}/history at the same prefix level.
    let tle_router = Router::new()
        .route("/", get(api::tle::list_tles))
        .route("/:norad_id", get(api::tle::get_tle))
        .route("/:norad_id/history", get(api::tle::get_tle_history));

    let app = Router::new()
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
        .fallback(fallback_404)
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    tracing::info!("orbidata listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
