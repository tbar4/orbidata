use serde::{Deserialize, Serialize};

/// Normalized CDM (CCSDS Conjunction Data Message) record.
/// Mirrors the key fields from Space-Track CDM JSON format.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConjunctionRecord {
    pub cdm_id: String,
    pub created: String,
    pub emergency_reportable: Option<String>,
    pub tca: String,
    pub miss_distance_m: f64,
    pub probability_of_collision: Option<f64>,
    pub sat1_id: u32,
    pub sat1_name: String,
    pub sat2_id: u32,
    pub sat2_name: String,
    pub sat1_object_type: Option<String>,
    pub sat2_object_type: Option<String>,
    pub collision_percentile: Option<f64>,
    pub source: ConjunctionSource,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ConjunctionSource {
    SpaceTrack,
    Sample,
}

/// Sample CDMs used when Space-Track credentials are not configured.
pub fn sample_conjunctions() -> Vec<ConjunctionRecord> {
    vec![
        ConjunctionRecord {
            cdm_id: "CDM-2026-001-SAMPLE".to_string(),
            created: "2026-04-08T00:00:00Z".to_string(),
            emergency_reportable: Some("N".to_string()),
            tca: "2026-04-10T14:23:00Z".to_string(),
            miss_distance_m: 312.5,
            probability_of_collision: Some(1.2e-4),
            sat1_id: 25544,
            sat1_name: "ISS (ZARYA)".to_string(),
            sat2_id: 48274,
            sat2_name: "COSMOS 1408 DEB".to_string(),
            sat1_object_type: Some("PAYLOAD".to_string()),
            sat2_object_type: Some("DEBRIS".to_string()),
            collision_percentile: Some(97.3),
            source: ConjunctionSource::Sample,
        },
        ConjunctionRecord {
            cdm_id: "CDM-2026-002-SAMPLE".to_string(),
            created: "2026-04-08T00:00:00Z".to_string(),
            emergency_reportable: Some("N".to_string()),
            tca: "2026-04-11T07:45:00Z".to_string(),
            miss_distance_m: 890.0,
            probability_of_collision: Some(3.4e-5),
            sat1_id: 43013,
            sat1_name: "STARLINK-1130".to_string(),
            sat2_id: 16908,
            sat2_name: "COSMOS 1953".to_string(),
            sat1_object_type: Some("PAYLOAD".to_string()),
            sat2_object_type: Some("PAYLOAD".to_string()),
            collision_percentile: Some(78.1),
            source: ConjunctionSource::Sample,
        },
    ]
}
