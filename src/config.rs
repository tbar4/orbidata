use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "orbidata", about = "Normalized TLE + CDM orbital data API")]
pub struct Config {
    #[arg(long, env = "HOST", default_value = "0.0.0.0")]
    pub host: String,

    #[arg(long, env = "PORT", default_value = "8080")]
    pub port: u16,

    /// Space-Track.org username (optional — enables CDM enrichment)
    #[arg(long, env = "SPACETRACK_USERNAME")]
    pub spacetrack_username: Option<String>,

    /// Space-Track.org password
    #[arg(long, env = "SPACETRACK_PASSWORD")]
    pub spacetrack_password: Option<String>,

    /// TLE cache TTL in seconds
    #[arg(long, env = "TLE_CACHE_TTL_SECS", default_value = "300")]
    pub tle_cache_ttl_secs: u64,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "RUST_LOG", default_value = "info")]
    pub log_level: String,
}
