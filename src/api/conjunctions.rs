use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::{error::AppError, ingest::cdm::fetch_conjunctions, state::AppState};

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
