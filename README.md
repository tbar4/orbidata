# orbidata

**Normalized TLE + CDM orbital data API** — commercial SDA data pipeline in Rust.

[![CI](https://github.com/tbar4/orbidata/actions/workflows/ci.yml/badge.svg)](https://github.com/tbar4/orbidata/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![CCSDS OMM](https://img.shields.io/badge/CCSDS-OMM_v2-005B9E)](https://public.ccsds.org/Pubs/502x0b3e1.pdf)
[![Space-Track](https://img.shields.io/badge/Space--Track-Integrated-1a3a5c)](https://www.space-track.org)
[![Endpoints](https://img.shields.io/badge/endpoints-6-20808D)](https://github.com/tbar4/orbidata#endpoints)

| | |
|---|---|
| **Data sources** | CelesTrak GP JSON · Space-Track `cdm_public` · Space-Track `gp_history` |
| **Schema** | CCSDS OMM v2 — field names compatible with SGP4/SDP4 propagators |
| **Auth** | Cookie session · 60 s rate-limit backoff · auto re-auth on 401/403 |
| **Stack** | Rust · axum · tokio · moka async cache · reqwest (rustls) |

---

## Why orbidata

The SDA data ecosystem is fragmented by design:

- **TLEs** come from CelesTrak in CCSDS OMM JSON — different schema from Space-Track's OMM endpoint
- **CDMs** live on Space-Track behind form-auth, with string-typed numeric fields and its own schema
- **Orbital history** is buried in `gp_history`, queryable only via Space-Track's REST DSL

Every operator, insurer, and analytics team builds the same bespoke integration stack. orbidata absorbs that complexity once — one normalized API, one CCSDS-aligned schema, commercial SLAs.

> **The Weather Company for orbital data.** One endpoint, not three portals.

---

## Live Demo

The examples below show exact API responses. The server ingests directly from CelesTrak and Space-Track at runtime.

### ISS — Current Orbital State

```bash
$ curl http://localhost:8080/v1/tle/25544
```

```json
{
  "data": {
    "norad_id": 25544,
    "name": "ISS (ZARYA)",
    "object_id": "1998-067A",
    "object_type": "PAYLOAD",
    "epoch": "2026-04-09T17:00:00.000000",
    "elements": {
      "mean_motion_rev_per_day": 15.49582900,
      "eccentricity": 0.00038200,
      "inclination_deg": 51.6416,
      "raan_deg": 97.2451,
      "arg_of_pericenter_deg": 86.3814,
      "mean_anomaly_deg": 273.8921,
      "bstar": 0.00022000,
      "semimajor_axis_km": 6797.50,
      "period_min": 92.89,
      "apoapsis_km": 423.10,
      "periapsis_km": 416.40
    },
    "tle": {
      "line1": "1 25544U 98067A   26099.70833333  .00012500  00000-0  22000-3 0  9993",
      "line2": "2 25544  51.6416  97.2451 0003820  86.3814 273.8921 15.49582900492830"
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

### Conjunction Screening — Active Events

```bash
$ curl http://localhost:8080/v1/conjunctions | jq '.data[0]'
```

```json
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
```

> Configure `SPACETRACK_USERNAME` + `SPACETRACK_PASSWORD` and hit `/v1/conjunctions/live` for real-time CDMs.

### ISS Orbital History — Epoch Timeline

```bash
$ curl "http://localhost:8080/v1/tle/25544/history?start=2026-04-01&end=2026-04-09&limit=3" \
  | jq '{window: .date_range, total: .total_epochs, first_epoch: .epochs[0].epoch, last_epoch: .epochs[-1].epoch}'
```

```json
{
  "window": {
    "earliest": "2026-04-01T06:14:22.000000",
    "latest": "2026-04-09T09:32:47.000000"
  },
  "total": 3,
  "first_epoch": "2026-04-01T06:14:22.000000",
  "last_epoch": "2026-04-09T09:32:47.000000"
}
```

> Each epoch is a full normalized OMM record. Feed `mean_motion_rev_per_day`, `eccentricity`, `inclination_deg`, `raan_deg`, `arg_of_pericenter_deg`, `mean_anomaly_deg`, and `bstar` into any SGP4/SDP4 propagator.

### Fleet-Wide Catalog — Active Satellites

```bash
$ curl "http://localhost:8080/v1/tle?page=1&per_page=3" \
  | jq '{total: .meta.total, page: .meta.page, objects: [.data[] | {id: .norad_id, name: .name, alt_km: .elements.apoapsis_km}]}'
```

```json
{
  "total": 10847,
  "page": 1,
  "objects": [
    { "id": 25544,  "name": "ISS (ZARYA)",       "alt_km": 423.10 },
    { "id": 48274,  "name": "COSMOS 1408 DEB",   "alt_km": 469.30 },
    { "id": 43013,  "name": "STARLINK-1130",      "alt_km": 556.80 }
  ]
}
```

---

## Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/v1/health` | — | Service health |
| `GET` | `/v1/tle` | — | Active satellites, paginated (`page`, `per_page`) |
| `GET` | `/v1/tle/{norad_id}` | — | Single satellite by NORAD ID |
| `GET` | `/v1/tle/{norad_id}/history` | Space-Track | Historical epochs from `gp_history` (`limit`, `start`, `end`) |
| `GET` | `/v1/conjunctions` | — | CDM events (sample schema, no auth) |
| `GET` | `/v1/conjunctions/live` | Space-Track | Real-time CDMs from Space-Track |

All endpoints return `application/json`. Error responses follow `{"error": {"code": N, "message": "..."}}`.

---

## Quick Start

```bash
git clone https://github.com/tbar4/orbidata.git
cd orbidata
cp .env.example .env
cargo run
```

Server starts on `http://localhost:8080`. No credentials required for TLE endpoints.

**Enable live CDM + history:**

```bash
# .env
SPACETRACK_USERNAME=your_username
SPACETRACK_PASSWORD=your_password
```

Register free at [space-track.org](https://www.space-track.org).

---

## Architecture

```
CelesTrak GP JSON ──► ingest/celestrak.rs ──► OrbitalElement ──► GET /v1/tle
                                                                  GET /v1/tle/{id}
                                               (moka TTL cache)

Space-Track gp_history ─► ingest/spacetrack.rs ─► OrbitalElement ──► GET /v1/tle/{id}/history
Space-Track cdm_public ──► ingest/spacetrack.rs ─► ConjunctionRecord ► GET /v1/conjunctions/live
                            (cookie auth, rate-limit tracker, auto re-auth)

Sample data ─────────────────────────────────► ConjunctionRecord ──► GET /v1/conjunctions
```

**Data flow:**

1. **Ingest** — CelesTrak is fetched on first request and cached for 5 minutes (configurable). Space-Track sessions are authenticated via `POST /ajaxauth/login`, cookies stored in-process for 90 minutes.
2. **Normalize** — Raw CCSDS OMM JSON → `OrbitalElement` struct. Space-Track string-typed CDM fields parsed to f64/u32 defensively.
3. **Rate limiting** — 60-second sliding window; warning at 28/30 req; 60 s backoff on 429; session invalidated on 401/403.
4. **Serve** — axum handlers return CCSDS-aligned JSON with structured error handling, CORS, and gzip compression.

---

## Space-Track Integration

### CDM Live Pull

```bash
# Real-time conjunction events (requires credentials)
curl http://localhost:8080/v1/conjunctions/live
```

- Authenticates via `POST /ajaxauth/login` (form-encoded, cookie response)
- Queries `cdm_public` class ordered by TCA ascending, limit 20
- Re-authenticates automatically on session expiry (90-minute TTL)
- Falls back: `/v1/conjunctions` always available without credentials

### Historical TLE Epochs

```bash
# Last 30 epochs (newest-first, no auth date window)
curl http://localhost:8080/v1/tle/25544/history

# Chronological window — correct ordering for propagation loops
curl "http://localhost:8080/v1/tle/25544/history?start=2026-01-01&end=2026-04-09&limit=100"
```

- Queries `gp_history` class — full CCSDS OMM JSON per epoch (not raw TLE strings)
- Date range triggers `orderby/EPOCH asc`; no range defaults to `orderby/EPOCH desc`
- Empty result → 404; no credentials → 503 with config instructions

### Rate Limit Behavior

| Condition | Behavior |
|-----------|----------|
| < 28 req/min | Normal operation |
| ≥ 28 req/min | Warning logged |
| HTTP 429 | 60 s backoff, 503 returned to client |
| HTTP 401/403 | Session invalidated, re-auth on next request |

---

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `8080` | Bind port |
| `RUST_LOG` | `info` | Log level (`trace` · `debug` · `info` · `warn`) |
| `TLE_CACHE_TTL_SECS` | `300` | CelesTrak TLE cache TTL |
| `SPACETRACK_USERNAME` | — | Enables `/v1/conjunctions/live` and `/v1/tle/{id}/history` |
| `SPACETRACK_PASSWORD` | — | Space-Track password |

---

## Roadmap

- [x] Normalized TLE API from CelesTrak GP JSON
- [x] Space-Track CDM live pull — `/v1/conjunctions/live`
- [x] Historical TLE epoch archive — `/v1/tle/{norad_id}/history`
- [ ] Space weather integration (NOAA SWPC) — v0.2
- [ ] Orbit propagation via SGP4/SDP4 — v0.3
- [ ] Rate limiting + API key auth — v0.3
- [ ] OpenAPI / Swagger docs — v0.3
- [ ] Docker image + Helm chart — v0.3
- [ ] WebSocket streaming for real-time conjunction alerts — v0.5
- [ ] Orbital regime segmentation (LEO / MEO / GEO / HEO) — v0.5

---

## License

[MIT](LICENSE)

## About

Built by [Trevor Barnes](https://github.com/tbar4) — Data Engineering Manager with a Master of Space Studies (University of North Dakota, in progress). Focused on commercial SDA data infrastructure for smallsat operators, insurers, and space analytics startups.
