use crate::error::AppError;
use crate::models::orbital_element::{CelesTrakGp, OrbitalElement};
use reqwest::Client;
use tracing::instrument;

const CELESTRAK_GP_BASE: &str = "https://celestrak.org/NORAD/elements/gp.php";

#[instrument(skip(client))]
pub async fn fetch_active_satellites(client: &Client) -> Result<Vec<OrbitalElement>, AppError> {
    let url = format!("{}?GROUP=active&FORMAT=json", CELESTRAK_GP_BASE);
    tracing::info!("Fetching active satellites from CelesTrak");
    let gp_records: Vec<CelesTrakGp> = client.get(&url).send().await?.json().await?;
    Ok(gp_records.into_iter().map(OrbitalElement::from).collect())
}

#[instrument(skip(client))]
pub async fn fetch_by_norad_id(
    client: &Client,
    norad_id: u32,
) -> Result<Option<OrbitalElement>, AppError> {
    let url = format!("{}?CATNR={}&FORMAT=json", CELESTRAK_GP_BASE, norad_id);
    tracing::info!(norad_id, "Fetching TLE by NORAD ID from CelesTrak");
    let gp_records: Vec<CelesTrakGp> = client.get(&url).send().await?.json().await?;
    Ok(gp_records.into_iter().next().map(OrbitalElement::from))
}
