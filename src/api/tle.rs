use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde_json::{json, Value};

use crate::{
    error::AppError,
    ingest::celestrak::{fetch_active_satellites, fetch_by_norad_id},
    models::pagination::{PaginatedResponse, PaginationParams},
    state::AppState,
};

pub async fn list_tles(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, AppError> {
    let satellites = match state.get_cached_tles().await {
        Some(cached) => {
            tracing::debug!("TLE cache hit");
            (*cached).clone()
        }
        None => {
            tracing::info!("TLE cache miss — fetching from CelesTrak");
            let fresh = fetch_active_satellites(&state.http_client).await?;
            state.set_cached_tles(fresh.clone()).await;
            fresh
        }
    };

    let page = params.page();
    let per_page = params.per_page();
    let paginated = PaginatedResponse::new(satellites, page, per_page);

    Ok(Json(json!({
        "data": paginated.data,
        "meta": paginated.meta,
    })))
}

pub async fn get_tle(
    State(state): State<AppState>,
    Path(norad_id): Path<u32>,
) -> Result<Json<Value>, AppError> {
    let element = fetch_by_norad_id(&state.http_client, norad_id).await?;

    match element {
        Some(e) => Ok(Json(json!({ "data": e }))),
        None => Err(AppError::NotFound(format!(
            "No TLE found for NORAD ID {}",
            norad_id
        ))),
    }
}
