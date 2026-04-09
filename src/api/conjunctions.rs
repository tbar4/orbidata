use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::{
    error::AppError, ingest::cdm::fetch_conjunctions, models::conjunction::ConjunctionRecord,
    state::AppState,
};

pub async fn list_conjunctions(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let conjunctions = fetch_conjunctions(
        state.config.spacetrack_username.as_deref(),
        state.config.spacetrack_password.as_deref(),
    )
    .await?;

    let total = conjunctions.len();
    let has_credentials = state.config.spacetrack_username.is_some();

    Ok(Json(json!({
        "data": conjunctions,
        "meta": {
            "total": total,
            "source": if has_credentials { "space-track" } else { "sample" },
            "note": if !has_credentials {
                Some("Configure SPACETRACK_USERNAME and SPACETRACK_PASSWORD for live CDM data")
            } else {
                None::<&str>
            }
        }
    })))
}

/// GET /v1/conjunctions/live — fetches real-time CDMs from Space-Track (requires credentials).
pub async fn list_conjunctions_live(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    match &state.spacetrack {
        None => Err(AppError::Unavailable(
            "Space-Track credentials not configured. Set SPACETRACK_USERNAME and SPACETRACK_PASSWORD to enable live CDM data.".to_string(),
        )),
        Some(client) => {
            let raw_cdms = client.fetch_cdms().await.map_err(AppError::Internal)?;
            let total = raw_cdms.len();
            let conjunctions: Vec<ConjunctionRecord> = raw_cdms
                .into_iter()
                .map(ConjunctionRecord::from_spacetrack)
                .collect();
            Ok(Json(json!({
                "data": conjunctions,
                "meta": {
                    "total": total,
                    "source": "space-track",
                    "live": true,
                }
            })))
        }
    }
}
