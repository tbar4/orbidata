# orbidata

**Normalized TLE + CDM orbital data API** — a commercial SDA data pipeline built in Rust.

[![CI](https://github.com/tbar4/orbidata/actions/workflows/ci.yml/badge.svg)](https://github.com/tbar4/orbidata/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

## Why orbidata

The Space Domain Awareness (SDA) data ecosystem is fragmented:

- **TLEs** live on CelesTrak with CCSDS OMM JSON, but require parsing and normalization
- **CDMs** (Conjunction Data Messages) live on Space-Track behind authentication, with their own schema
- **Space weather** data sits on NOAA SWPC in yet another format

Each source has different schemas, rate limits, and auth models. Operators, insurers, and analytics teams waste engineering cycles building bespoke integrations for each one.

**orbidata** provides a single normalized REST API that unifies these sources under a consistent CCSDS-aligned JSON schema. Built in Rust for memory safety and performance in safety-critical orbital data pipelines.

## Endpoints

| Method | Path                  | Description                        |
|--------|-----------------------|------------------------------------|
| GET    | `/v1/health`          | Service health check               |
| GET    | `/v1/tle`             | List active satellites (paginated) |
| GET    | `/v1/tle/{norad_id}`  | Get single satellite by NORAD ID   |
| GET    | `/v1/conjunctions`    | List conjunction events (CDMs)     |

## Quick Start

### Prerequisites

- Rust 1.75+ ([install](https://rustup.rs))

### Build and run

```bash
git clone https://github.com/tbar4/orbidata.git
cd orbidata
cp .env.example .env
cargo run
```

The server starts on `http://localhost:8080` by default.

### Example requests

```bash
# Health check
curl http://localhost:8080/v1/health

# Get ISS TLE by NORAD ID
curl http://localhost:8080/v1/tle/25544

# List active satellites (paginated)
curl "http://localhost:8080/v1/tle?page=1&per_page=20"

# List conjunction events
curl http://localhost:8080/v1/conjunctions
```

## Data Schema

### Normalized Orbital Element (TLE response)

```json
{
  "data": {
    "norad_id": 25544,
    "name": "ISS (ZARYA)",
    "object_id": "1998-067A",
    "object_type": "PAYLOAD",
    "epoch": "2026-04-08T12:00:00.000000",
    "elements": {
      "mean_motion_rev_per_day": 15.50104,
      "eccentricity": 0.0006703,
      "inclination_deg": 51.6416,
      "raan_deg": 247.4627,
      "arg_of_pericenter_deg": 130.5360,
      "mean_anomaly_deg": 325.0288,
      "bstar": 0.000036771,
      "semimajor_axis_km": 6797.22,
      "period_min": 92.89,
      "apoapsis_km": 423.64,
      "periapsis_km": 414.50
    },
    "tle": {
      "line1": "1 25544U 98067A   26098.50000000  .00016717  00000-0  36771-4 0  9991",
      "line2": "2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.50104000    09"
    },
    "metadata": {
      "country_code": "ISS",
      "launch_date": "1998-11-20",
      "decay_date": null,
      "rcs_size": "LARGE",
      "site": "TYMSC"
    }
  }
}
```

### Conjunction Record (CDM response)

```json
{
  "data": [
    {
      "cdm_id": "CDM-2026-001-SAMPLE",
      "created": "2026-04-08T00:00:00Z",
      "emergency_reportable": "N",
      "tca": "2026-04-10T14:23:00Z",
      "miss_distance_m": 312.5,
      "probability_of_collision": 0.00012,
      "sat1_id": 25544,
      "sat1_name": "ISS (ZARYA)",
      "sat2_id": 48274,
      "sat2_name": "COSMOS 1408 DEB",
      "sat1_object_type": "PAYLOAD",
      "sat2_object_type": "DEBRIS",
      "collision_percentile": 97.3,
      "source": "sample"
    }
  ],
  "meta": {
    "total": 2,
    "source": "sample",
    "note": "Configure SPACETRACK_USERNAME and SPACETRACK_PASSWORD for live CDM data"
  }
}
```

## Space-Track CDM Integration

To enable live Conjunction Data Messages from Space-Track:

1. Register for a free account at [space-track.org](https://www.space-track.org)
2. Set environment variables:
   ```bash
   export SPACETRACK_USERNAME=your_username
   export SPACETRACK_PASSWORD=your_password
   ```
3. Restart the server — the `/v1/conjunctions` endpoint will pull live CDM data

Without credentials, the API returns well-structured sample data that mirrors the exact CDM schema, so you can develop and test integrations immediately.

## Architecture

```
CelesTrak GP JSON ──► ingest/celestrak.rs ──► OrbitalElement ──► GET /v1/tle
                                                                  (paginated, cached)

Space-Track CDM ────► ingest/cdm.rs ───────► ConjunctionRecord ► GET /v1/conjunctions
(or sample data)
```

**Data flow:**

1. **Ingest** — HTTP clients fetch raw CCSDS OMM JSON from CelesTrak or CDM records from Space-Track
2. **Normalize** — Raw upstream schemas are converted to consistent Rust structs with clear field naming
3. **Cache** — TLE data is cached in-memory with configurable TTL (default 5 minutes) using Moka async cache
4. **Serve** — Axum handlers return paginated, CCSDS-aligned JSON responses with structured error handling

## Configuration

| Environment Variable     | CLI Flag                | Default   | Description                       |
|--------------------------|-------------------------|-----------|-----------------------------------|
| `HOST`                   | `--host`                | `0.0.0.0` | Bind address                      |
| `PORT`                   | `--port`                | `8080`    | Bind port                         |
| `RUST_LOG`               | `--log-level`           | `info`    | Log level (trace/debug/info/warn) |
| `TLE_CACHE_TTL_SECS`    | `--tle-cache-ttl-secs`  | `300`     | TLE cache TTL in seconds          |
| `SPACETRACK_USERNAME`    | `--spacetrack-username` | —         | Space-Track.org username          |
| `SPACETRACK_PASSWORD`    | `--spacetrack-password` | —         | Space-Track.org password          |

All configuration can be set via environment variables or CLI flags. The server reads `.env` files if present.

## Roadmap

- [ ] Space weather integration (NOAA SWPC)
- [ ] Orbit propagation via SGP4/SDP4 (satellite position at T+n minutes)
- [ ] Space-Track CDM live pull (full authentication + query)
- [ ] Rate limiting and API key authentication
- [ ] OpenAPI / Swagger documentation
- [ ] Docker image and Helm chart
- [ ] WebSocket streaming for real-time conjunction alerts
- [ ] Historical TLE archive and diff tracking

## License

[MIT](LICENSE)

## About

Built by [Trevor Barnes](https://github.com/tbar4) — Data Engineering Manager with a Master of Space Studies (University of North Dakota, in progress). Focused on commercial SDA data infrastructure for smallsat operators, insurers, and space analytics startups.
