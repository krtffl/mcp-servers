//! Integration tests for mcp-motorsport tools.
//!
//! Each test spins up a `wiremock::MockServer` to simulate the `OpenF1` / Jolpica-F1
//! APIs, then calls the tool `execute` functions directly and asserts on the results.

use std::time::Duration;

use mcp_common::ResponseCache;
use mcp_motorsport::config::{JolpicaConfig, OpenF1Config};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Build an `OpenF1Config` pointing at the given mock server URI.
fn openf1_config(mock_uri: &str) -> OpenF1Config {
    OpenF1Config {
        base_url: mock_uri.to_owned(),
        rate_limit_rps: 100.0,
        auth_token: None,
    }
}

/// Build a fresh `ResponseCache` with short TTL for test isolation.
fn test_cache() -> ResponseCache {
    ResponseCache::new(100, Duration::from_mins(1))
}

/// Standard mock response for `GET /sessions` returning a single session.
fn sessions_json() -> serde_json::Value {
    serde_json::json!([{
        "session_key": 9001,
        "session_name": "Race",
        "country_name": "Bahrain",
        "year": 2024
    }])
}

/// Standard mock response for `GET /drivers` returning two drivers.
fn drivers_json() -> serde_json::Value {
    serde_json::json!([
        {
            "driver_number": 1,
            "name_acronym": "VER",
            "full_name": "Max VERSTAPPEN",
            "first_name": "Max",
            "last_name": "VERSTAPPEN",
            "team_name": "Red Bull Racing"
        },
        {
            "driver_number": 4,
            "name_acronym": "NOR",
            "full_name": "Lando NORRIS",
            "first_name": "Lando",
            "last_name": "NORRIS",
            "team_name": "McLaren"
        }
    ])
}

/// Mount the standard `/sessions` mock for Bahrain 2024 Race.
async fn mount_sessions(mock: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/sessions"))
        .and(query_param("year", "2024"))
        .and(query_param("country_name", "Bahrain"))
        .and(query_param("session_name", "Race"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sessions_json()))
        .mount(mock)
        .await;
}

/// Mount the standard `/drivers` mock for `session_key`=9001.
async fn mount_drivers(mock: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/drivers"))
        .and(query_param("session_key", "9001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(drivers_json()))
        .mount(mock)
        .await;
}

// ---------------------------------------------------------------------------
// Test 1: resolve_session_key
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_resolve_session_key() {
    let mock = MockServer::start().await;
    mount_sessions(&mock).await;

    let config = openf1_config(&mock.uri());
    let client = reqwest::Client::new();
    let cache = test_cache();

    let result = mcp_motorsport::tools::common::resolve_session_key(
        2024, "Bahrain", "Race", &client, &cache, &config,
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got: {result:?}");
    assert_eq!(result.unwrap(), 9001);
}

// ---------------------------------------------------------------------------
// Test 2: resolve_driver_number — by acronym and by partial name
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_resolve_driver_number_by_acronym() {
    let mock = MockServer::start().await;
    mount_drivers(&mock).await;

    let config = openf1_config(&mock.uri());
    let client = reqwest::Client::new();
    let cache = test_cache();

    let result =
        mcp_motorsport::tools::common::resolve_driver_number(9001, "VER", &client, &cache, &config)
            .await;

    assert!(result.is_ok(), "expected Ok, got: {result:?}");
    assert_eq!(result.unwrap(), 1);
}

#[tokio::test]
async fn test_resolve_driver_number_by_name() {
    let mock = MockServer::start().await;
    mount_drivers(&mock).await;

    let config = openf1_config(&mock.uri());
    let client = reqwest::Client::new();
    let cache = test_cache();

    // "Norris" should match via full_name contains (case-insensitive).
    let result = mcp_motorsport::tools::common::resolve_driver_number(
        9001, "Norris", &client, &cache, &config,
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got: {result:?}");
    assert_eq!(result.unwrap(), 4);
}

// ---------------------------------------------------------------------------
// Test 3: get_lap_times
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_get_lap_times() {
    let mock = MockServer::start().await;
    mount_sessions(&mock).await;
    mount_drivers(&mock).await;

    Mock::given(method("GET"))
        .and(path("/laps"))
        .and(query_param("session_key", "9001"))
        .and(query_param("driver_number", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "lap_number": 1,
                "duration_sector_1": 28.5,
                "duration_sector_2": 34.2,
                "duration_sector_3": 31.1,
                "lap_duration": 93.8,
                "driver_number": 1,
                "is_pit_out_lap": false
            },
            {
                "lap_number": 2,
                "duration_sector_1": 27.9,
                "duration_sector_2": 33.8,
                "duration_sector_3": 30.5,
                "lap_duration": 92.2,
                "driver_number": 1,
                "is_pit_out_lap": false
            }
        ])))
        .mount(&mock)
        .await;

    let config = openf1_config(&mock.uri());
    let client = reqwest::Client::new();
    let cache = test_cache();

    let result = mcp_motorsport::tools::lap_times::execute(
        2024, "Bahrain", "Race", "VER", &client, &cache, &config,
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got: {result:?}");

    let body: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(body["laps_count"], 2);
    assert_eq!(body["laps"].as_array().unwrap().len(), 2);
    assert_eq!(body["laps"][0]["lap_number"], 1);
    assert_eq!(body["laps"][1]["lap_number"], 2);
}

// ---------------------------------------------------------------------------
// Test 4: get_tire_stints
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_get_tire_stints() {
    let mock = MockServer::start().await;
    mount_sessions(&mock).await;
    mount_drivers(&mock).await;

    Mock::given(method("GET"))
        .and(path("/stints"))
        .and(query_param("session_key", "9001"))
        .and(query_param("driver_number", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "driver_number": 1,
                "stint_number": 1,
                "compound": "MEDIUM",
                "lap_start": 1,
                "lap_end": 20,
                "tyre_age_at_start": 0
            },
            {
                "driver_number": 1,
                "stint_number": 2,
                "compound": "HARD",
                "lap_start": 21,
                "lap_end": 57,
                "tyre_age_at_start": 0
            }
        ])))
        .mount(&mock)
        .await;

    let config = openf1_config(&mock.uri());
    let client = reqwest::Client::new();
    let cache = test_cache();

    let result = mcp_motorsport::tools::tire_stints::execute(
        2024,
        "Bahrain",
        Some("VER"),
        &client,
        &cache,
        &config,
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got: {result:?}");

    let body: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(body["stints_count"], 2);

    let stints = body["stints"].as_array().unwrap();
    assert_eq!(stints.len(), 2);
    assert_eq!(stints[0]["compound"], "MEDIUM");
    assert_eq!(stints[0]["lap_start"], 1);
    assert_eq!(stints[0]["lap_end"], 20);
    assert_eq!(stints[1]["compound"], "HARD");
    assert_eq!(stints[1]["lap_start"], 21);
    assert_eq!(stints[1]["lap_end"], 57);
}

// ---------------------------------------------------------------------------
// Test 5: get_standings (Jolpica-F1 API)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_get_standings() {
    let mock = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/2024/driverStandings.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "MRData": {
                "StandingsTable": {
                    "StandingsLists": [{
                        "DriverStandings": [
                            {
                                "position": "1",
                                "points": "575",
                                "wins": "19",
                                "Driver": {
                                    "givenName": "Max",
                                    "familyName": "Verstappen",
                                    "code": "VER"
                                },
                                "Constructors": [{"name": "Red Bull"}]
                            }
                        ]
                    }]
                }
            }
        })))
        .mount(&mock)
        .await;

    let jolpica = JolpicaConfig {
        base_url: mock.uri(),
    };
    let client = reqwest::Client::new();
    let cache = test_cache();

    let result =
        mcp_motorsport::tools::standings::execute(2024, "drivers", &client, &cache, &jolpica).await;

    assert!(result.is_ok(), "expected Ok, got: {result:?}");

    let body: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(body["standings_count"], 1);

    let standings = body["standings"].as_array().unwrap();
    assert_eq!(standings.len(), 1);
    assert_eq!(standings[0]["driver"], "Max Verstappen");
    assert_eq!(standings[0]["points"], "575");
    assert_eq!(standings[0]["wins"], "19");
    assert_eq!(standings[0]["team"], "Red Bull");
}

// ---------------------------------------------------------------------------
// Test 6: compare_drivers
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_compare_drivers() {
    let mock = MockServer::start().await;
    mount_sessions(&mock).await;
    mount_drivers(&mock).await;

    // Laps for driver 1 (VER)
    Mock::given(method("GET"))
        .and(path("/laps"))
        .and(query_param("session_key", "9001"))
        .and(query_param("driver_number", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"lap_number": 1, "lap_duration": 93.8, "driver_number": 1},
            {"lap_number": 2, "lap_duration": 92.2, "driver_number": 1},
            {"lap_number": 3, "lap_duration": 91.5, "driver_number": 1}
        ])))
        .mount(&mock)
        .await;

    // Laps for driver 4 (NOR)
    Mock::given(method("GET"))
        .and(path("/laps"))
        .and(query_param("session_key", "9001"))
        .and(query_param("driver_number", "4"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"lap_number": 1, "lap_duration": 94.1, "driver_number": 4},
            {"lap_number": 2, "lap_duration": 92.5, "driver_number": 4},
            {"lap_number": 3, "lap_duration": 91.8, "driver_number": 4}
        ])))
        .mount(&mock)
        .await;

    let config = openf1_config(&mock.uri());
    let client = reqwest::Client::new();
    let cache = test_cache();

    let result = mcp_motorsport::tools::compare_drivers::execute(
        2024, "Bahrain", "Race", "VER", "NOR", &client, &cache, &config,
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got: {result:?}");

    let body: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(body["comparable_laps"], 3);
    assert_eq!(body["driver_a"], "VER");
    assert_eq!(body["driver_b"], "NOR");
    assert_eq!(body["driver_a_number"], 1);
    assert_eq!(body["driver_b_number"], 4);

    // VER was faster on every lap, so all deltas should be negative (A - B < 0).
    let deltas = body["lap_deltas"].as_array().unwrap();
    assert_eq!(deltas.len(), 3);
    for delta in deltas {
        let d = delta["delta_seconds"].as_f64().unwrap();
        assert!(d < 0.0, "expected negative delta (VER faster), got {d}");
    }

    // Average pace: VER avg = (93.8+92.2+91.5)/3 = 92.5, NOR avg = (94.1+92.5+91.8)/3 = 92.8
    let avg = &body["average_pace"];
    assert!(avg["driver_a"].as_f64().unwrap() < avg["driver_b"].as_f64().unwrap());
    assert!(avg["delta"].as_f64().unwrap() < 0.0);

    // Best laps: VER 91.5, NOR 91.8
    let best = &body["best_lap"];
    assert!((best["driver_a"].as_f64().unwrap() - 91.5).abs() < 0.01);
    assert!((best["driver_b"].as_f64().unwrap() - 91.8).abs() < 0.01);
}
