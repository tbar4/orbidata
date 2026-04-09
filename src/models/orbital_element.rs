use serde::{Deserialize, Serialize};

/// Raw CCSDS OMM JSON from CelesTrak GP API
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CelesTrakGp {
    #[serde(rename = "OBJECT_NAME")]
    pub object_name: String,
    #[serde(rename = "OBJECT_ID")]
    pub object_id: Option<String>,
    #[serde(rename = "NORAD_CAT_ID")]
    pub norad_cat_id: u32,
    #[serde(rename = "OBJECT_TYPE")]
    pub object_type: Option<String>,
    #[serde(rename = "EPOCH")]
    pub epoch: String,
    #[serde(rename = "MEAN_MOTION")]
    pub mean_motion: f64,
    #[serde(rename = "ECCENTRICITY")]
    pub eccentricity: f64,
    #[serde(rename = "INCLINATION")]
    pub inclination: f64,
    #[serde(rename = "RA_OF_ASC_NODE")]
    pub ra_of_asc_node: f64,
    #[serde(rename = "ARG_OF_PERICENTER")]
    pub arg_of_pericenter: f64,
    #[serde(rename = "MEAN_ANOMALY")]
    pub mean_anomaly: f64,
    #[serde(rename = "BSTAR")]
    pub bstar: f64,
    #[serde(rename = "MEAN_MOTION_DOT")]
    pub mean_motion_dot: Option<f64>,
    #[serde(rename = "MEAN_MOTION_DDOT")]
    pub mean_motion_ddot: Option<f64>,
    #[serde(rename = "SEMIMAJOR_AXIS")]
    pub semimajor_axis: Option<f64>,
    #[serde(rename = "PERIOD")]
    pub period: Option<f64>,
    #[serde(rename = "APOAPSIS")]
    pub apoapsis: Option<f64>,
    #[serde(rename = "PERIAPSIS")]
    pub periapsis: Option<f64>,
    #[serde(rename = "TLE_LINE1")]
    pub tle_line1: Option<String>,
    #[serde(rename = "TLE_LINE2")]
    pub tle_line2: Option<String>,
    #[serde(rename = "DECAY_DATE")]
    pub decay_date: Option<String>,
    #[serde(rename = "SITE")]
    pub site: Option<String>,
    #[serde(rename = "RCS_SIZE")]
    pub rcs_size: Option<String>,
    #[serde(rename = "COUNTRY_CODE")]
    pub country_code: Option<String>,
    #[serde(rename = "LAUNCH_DATE")]
    pub launch_date: Option<String>,
}

/// Normalized orbital element set returned by the API
#[derive(Debug, Serialize, Clone)]
pub struct OrbitalElement {
    pub norad_id: u32,
    pub name: String,
    pub object_id: Option<String>,
    pub object_type: Option<String>,
    pub epoch: String,
    pub elements: KeplerianElements,
    pub tle: Option<TleLines>,
    pub metadata: SatelliteMetadata,
}

#[derive(Debug, Serialize, Clone)]
pub struct KeplerianElements {
    /// Mean motion (revolutions per day)
    pub mean_motion_rev_per_day: f64,
    /// Eccentricity (dimensionless)
    pub eccentricity: f64,
    /// Inclination (degrees)
    pub inclination_deg: f64,
    /// Right ascension of ascending node (degrees)
    pub raan_deg: f64,
    /// Argument of pericenter (degrees)
    pub arg_of_pericenter_deg: f64,
    /// Mean anomaly (degrees)
    pub mean_anomaly_deg: f64,
    /// BSTAR drag term
    pub bstar: f64,
    /// Semi-major axis (km), if available
    pub semimajor_axis_km: Option<f64>,
    /// Orbital period (minutes), if available
    pub period_min: Option<f64>,
    /// Apoapsis altitude (km), if available
    pub apoapsis_km: Option<f64>,
    /// Periapsis altitude (km), if available
    pub periapsis_km: Option<f64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TleLines {
    pub line1: String,
    pub line2: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct SatelliteMetadata {
    pub country_code: Option<String>,
    pub launch_date: Option<String>,
    pub decay_date: Option<String>,
    pub rcs_size: Option<String>,
    pub site: Option<String>,
}

impl From<CelesTrakGp> for OrbitalElement {
    fn from(gp: CelesTrakGp) -> Self {
        let tle = match (gp.tle_line1, gp.tle_line2) {
            (Some(l1), Some(l2)) => Some(TleLines {
                line1: l1,
                line2: l2,
            }),
            _ => None,
        };

        Self {
            norad_id: gp.norad_cat_id,
            name: gp.object_name,
            object_id: gp.object_id,
            object_type: gp.object_type,
            epoch: gp.epoch,
            elements: KeplerianElements {
                mean_motion_rev_per_day: gp.mean_motion,
                eccentricity: gp.eccentricity,
                inclination_deg: gp.inclination,
                raan_deg: gp.ra_of_asc_node,
                arg_of_pericenter_deg: gp.arg_of_pericenter,
                mean_anomaly_deg: gp.mean_anomaly,
                bstar: gp.bstar,
                semimajor_axis_km: gp.semimajor_axis,
                period_min: gp.period,
                apoapsis_km: gp.apoapsis,
                periapsis_km: gp.periapsis,
            },
            tle,
            metadata: SatelliteMetadata {
                country_code: gp.country_code,
                launch_date: gp.launch_date,
                decay_date: gp.decay_date,
                rcs_size: gp.rcs_size,
                site: gp.site,
            },
        }
    }
}
