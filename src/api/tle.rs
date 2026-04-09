use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde_json::{json, Value};

use crate::{
    error::AppError,
    ingest::celestrak::{fetch_active_satellites, fetch_by_norad_id},
    models::{
        orbital_element::{DateRange, OrbitalElement, TleHistoryParams, TleHistoryResponse},
        pagination::{PaginatedResponse, PaginationParams},
    },
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

/// GET /v1/tle/:norad_id/history
/// Returns historical TLE epochs for a NORAD ID from Space-Track tle class.
/// Requires SPACETRACK_USERNAME and SPACETRACK_PASSWORD.
pub async fn get_tle_history(
    State(state): State<AppState>,
    Path(norad_id): Path<u32>,
    Query(params): Query<TleHistoryParams>,
) -> Result<Json<Value>, AppError> {
    tracing::info!(norad_id, "GET /v1/tle/{}/history called", norad_id);

    let client = state
        .spacetrack
        .as_ref()
        .ok_or_else(|| {
            tracing::warn!(
                norad_id,
                "Space-Track not configured — cannot serve TLE history for NORAD ID {}",
                norad_id
            );
            AppError::Unavailable(
                "Space-Track credentials not configured. Set SPACETRACK_USERNAME and SPACETRACK_PASSWORD in .env to enable TLE history.".to_string(),
            )
        })?;

    let limit = params.limit();
    let start = params.start.as_deref();
    let end = params.end.as_deref();

    let raw_records = client
        .fetch_tle_history(norad_id, limit, start, end)
        .await
        .map_err(AppError::Internal)?;

    if raw_records.is_empty() {
        return Err(AppError::NotFound(format!(
            "No historical TLE epochs found for NORAD ID {}",
            norad_id
        )));
    }

    let epochs: Vec<OrbitalElement> = raw_records.into_iter().map(OrbitalElement::from).collect();

    let total_epochs = epochs.len();

    let date_range = if total_epochs > 0 {
        let mut min_epoch = epochs[0].epoch.clone();
        let mut max_epoch = epochs[0].epoch.clone();
        for e in &epochs {
            if e.epoch < min_epoch {
                min_epoch = e.epoch.clone();
            }
            if e.epoch > max_epoch {
                max_epoch = e.epoch.clone();
            }
        }
        Some(DateRange {
            earliest: min_epoch,
            latest: max_epoch,
        })
    } else {
        None
    };

    let response = TleHistoryResponse {
        norad_id,
        name: epochs.first().map(|e| e.name.clone()).unwrap_or_default(),
        total_epochs,
        date_range,
        epochs,
        propagation_note: "Epochs are normalized CCSDS OMM records suitable for SGP4/SDP4 propagation. Feed mean_motion_rev_per_day, eccentricity, inclination_deg, raan_deg, arg_of_pericenter_deg, mean_anomaly_deg, and bstar into your propagator at each epoch.",
    };

    Ok(Json(
        serde_json::to_value(response).map_err(AppError::Serialization)?,
    ))
}
