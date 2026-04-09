use moka::future::Cache;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use orbidata::config::Config;
use orbidata::ingest::spacetrack::SpaceTrackClient;
use orbidata::state::AppState;

fn fake_tle_json() -> Value {
    serde_json::json!([{
        "CCSDS_OMM_VERS": "2.0",
        "COMMENT": "GENERATED VIA SPACE-TRACK.ORG API",
        "CLASSIFICATION_TYPE": "U",
        "NORAD_CAT_ID": "25544",
        "OBJECT_NAME": "ISS (ZARYA)",
        "OBJECT_ID": "1998-067A",
        "EPOCH": "2026-04-01T12:00:00.000000",
        "MEAN_MOTION": "15.48919799",
        "ECCENTRICITY": ".0001234",
        "INCLINATION": "51.6416",
        "RA_OF_ASC_NODE": "123.4567",
        "ARG_OF_PERICENTER": "45.6789",
        "MEAN_ANOMALY": "314.1592",
        "EPHEMERIS_TYPE": "0",
        "ELEMENT_SET_NO": "999",
        "REV_AT_EPOCH": "45678",
        "BSTAR": ".12345E-4",
        "MEAN_MOTION_DOT": ".123456E-4",
        "MEAN_MOTION_DDOT": "0",
        "SEMIMAJOR_AXIS": "6793.234",
        "PERIOD": "92.726",
        "APOAPSIS": "422.234",
        "PERIAPSIS": "408.234",
        "OBJECT_TYPE": "PAYLOAD",
        "RCS_SIZE": "LARGE",
        "COUNTRY_CODE": "ISS",
        "LAUNCH_DATE": "1998-11-20",
        "SITE": "TTMTR",
        "DECAY_DATE": null,
        "DECAYED": "0",
        "FILE": "3456789",
        "GP_ID": "234567890",
        "TLE_LINE0": "0 ISS (ZARYA)",
        "TLE_LINE1": "1 25544U 98067A   26091.50000000  .00001234  00000+0  12345-4 0  9999",
        "TLE_LINE2": "2 25544  51.6416 123.4567 0001234  45.6789 314.1592 15.48919799456789"
    }])
}

fn build_test_state(mock_base_url: String) -> AppState {
    let http_client = Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build test HTTP client");

    let tle_cache = Cache::builder()
        .max_capacity(10)
        .time_to_live(Duration::from_secs(300))
        .build();

    let spacetrack = SpaceTrackClient::with_base_url(
        mock_base_url,
        "testuser".to_string(),
        "testpass".to_string(),
    )
    .expect("Failed to build SpaceTrackClient");

    let config = Config {
        host: "127.0.0.1".to_string(),
        port: 0,
        spacetrack_username: Some("testuser".to_string()),
        spacetrack_password: Some("testpass".to_string()),
        tle_cache_ttl_secs: 300,
        log_level: "error".to_string(),
    };

    AppState {
        config: Arc::new(config),
        http_client,
        tle_cache,
        spacetrack: Some(Arc::new(spacetrack)),
    }
}

#[tokio::test]
async fn test_tle_history_returns_orbital_elements() {
    let mock_server = MockServer::start().await;

    // Stub: POST /ajaxauth/login -> 200 with session cookie
    Mock::given(method("POST"))
        .and(path("/ajaxauth/login"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("\"\"")
                .append_header("Set-Cookie", "chocolatechip=testtoken; Path=/"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    // Stub: GET TLE class query for NORAD 25544
    // Space-Track uses spaces in URL paths (e.g., "EPOCH desc") which reqwest encodes as %20
    Mock::given(method("GET"))
        .and(path(
            "/basicspacedata/query/class/tle/NORAD_CAT_ID/25544/orderby/EPOCH%20desc/limit/30/format/json",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fake_tle_json()))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Build the axum app with mock-backed state
    let state = build_test_state(mock_server.uri());
    let app = orbidata::build_app(state);

    // Bind to a random port and serve
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service()).await.unwrap();
    });

    // Make a real HTTP request to the running server
    let client = Client::new();
    let url = format!("http://{}/v1/tle/25544/history", addr);
    let resp = client.get(&url).send().await.expect("Request failed");

    assert_eq!(resp.status().as_u16(), 200);

    let body: Value = resp.json().await.expect("Response is valid JSON");

    // Verify response structure
    assert_eq!(body["norad_id"], 25544);
    assert_eq!(body["name"], "ISS (ZARYA)");
    assert_eq!(body["total_epochs"], 1);

    // Verify epochs array is non-empty
    let epochs = body["epochs"].as_array().expect("epochs is an array");
    assert_eq!(epochs.len(), 1);

    let epoch = &epochs[0];
    assert_eq!(epoch["norad_id"], 25544);
    assert_eq!(epoch["name"], "ISS (ZARYA)");
    assert_eq!(epoch["epoch"], "2026-04-01T12:00:00.000000");
    assert_eq!(epoch["object_id"], "1998-067A");
    assert_eq!(epoch["object_type"], "PAYLOAD");

    // Verify parsed numeric orbital elements
    let elements = &epoch["elements"];
    assert_eq!(elements["mean_motion_rev_per_day"], 15.48919799);
    assert_eq!(elements["eccentricity"], 0.0001234);
    assert_eq!(elements["inclination_deg"], 51.6416);
    assert_eq!(elements["raan_deg"], 123.4567);
    assert_eq!(elements["arg_of_pericenter_deg"], 45.6789);
    assert_eq!(elements["mean_anomaly_deg"], 314.1592);
    assert_eq!(elements["bstar"], 0.12345e-4);
    assert_eq!(elements["period_min"], 92.726);
    assert_eq!(elements["apoapsis_km"], 422.234);
    assert_eq!(elements["periapsis_km"], 408.234);

    // Verify TLE lines
    let tle = &epoch["tle"];
    assert_eq!(
        tle["line1"],
        "1 25544U 98067A   26091.50000000  .00001234  00000+0  12345-4 0  9999"
    );
    assert_eq!(
        tle["line2"],
        "2 25544  51.6416 123.4567 0001234  45.6789 314.1592 15.48919799456789"
    );

    // Verify metadata
    let metadata = &epoch["metadata"];
    assert_eq!(metadata["country_code"], "ISS");
    assert_eq!(metadata["launch_date"], "1998-11-20");
    assert_eq!(metadata["rcs_size"], "LARGE");
    assert_eq!(metadata["site"], "TTMTR");

    // Verify date range
    assert_eq!(
        body["date_range"]["earliest"],
        "2026-04-01T12:00:00.000000"
    );
    assert_eq!(body["date_range"]["latest"], "2026-04-01T12:00:00.000000");

    // Verify the mock server received exactly the expected calls
    mock_server.verify().await;
}
