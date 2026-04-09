use crate::error::AppError;
use crate::models::conjunction::{sample_conjunctions, ConjunctionRecord};
use tracing::instrument;

/// Fetch conjunctions. When Space-Track credentials are provided, this will
/// authenticate and pull real CDM data. Until credentials are configured,
/// returns well-structured sample data that mirrors the normalized CDM schema.
#[instrument]
pub async fn fetch_conjunctions(
    spacetrack_username: Option<&str>,
    _spacetrack_password: Option<&str>,
) -> Result<Vec<ConjunctionRecord>, AppError> {
    if spacetrack_username.is_none() {
        tracing::info!("No Space-Track credentials configured — returning sample CDM data");
        return Ok(sample_conjunctions());
    }

    // TODO: Implement Space-Track authentication and CDM pull
    // POST https://www.space-track.org/ajaxauth/login
    // GET  https://www.space-track.org/basicspacedata/query/class/cdm_public/...
    tracing::warn!("Space-Track CDM fetch not yet implemented — returning sample data");
    Ok(sample_conjunctions())
}
