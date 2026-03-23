//! Integration tests for mcp-infra tools.

use std::time::Duration;

use mcp_common::ResponseCache;
use mcp_infra::config::{AlertmanagerConfig, GrafanaConfig, PrometheusConfig};
use mcp_infra::tools;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_health_returns_valid_json() {
    let result = tools::health::execute();
    assert!(result.is_ok(), "health::execute() returned Err: {result:?}");

    let json_str = result.unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("health output should be valid JSON");

    assert!(
        parsed.get("cpu_usage_percent").is_some(),
        "missing cpu_usage_percent"
    );
    assert!(
        parsed.get("memory_used_mb").is_some(),
        "missing memory_used_mb"
    );
    assert!(parsed.get("hostname").is_some(), "missing hostname");
}

#[tokio::test]
async fn test_prometheus_query() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "data": {
                "resultType": "vector",
                "result": [
                    {
                        "metric": {"__name__": "up"},
                        "value": [1_616_000_000, "1"]
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let config = PrometheusConfig {
        url: mock_server.uri(),
        auth_token: None,
    };
    let client = reqwest::Client::new();
    let cache = ResponseCache::new(100, Duration::from_secs(60));

    let result =
        tools::prometheus::execute("up", None, None, None, None, &client, &cache, &config).await;

    assert!(result.is_ok(), "prometheus::execute() failed: {result:?}");

    let json_str = result.unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("prometheus output should be valid JSON");

    assert_eq!(parsed["status"], "success");
    assert_eq!(parsed["data"]["resultType"], "vector");
}

#[tokio::test]
async fn test_grafana_dashboard_search() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": 1,
                "uid": "abc123",
                "title": "System Overview",
                "type": "dash-db"
            }
        ])))
        .mount(&mock_server)
        .await;

    let config = GrafanaConfig {
        url: mock_server.uri(),
        auth_token: None,
    };
    let client = reqwest::Client::new();
    let cache = ResponseCache::new(100, Duration::from_secs(60));

    let result =
        tools::grafana::execute(Some("System"), None, &client, &cache, &config).await;

    assert!(result.is_ok(), "grafana::execute() failed: {result:?}");

    let json_str = result.unwrap();
    assert!(
        json_str.contains("System Overview"),
        "response should contain 'System Overview', got: {json_str}"
    );
}

#[tokio::test]
async fn test_alertmanager_alerts() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v2/alerts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "labels": {
                    "alertname": "HighCPU",
                    "severity": "critical"
                },
                "status": "firing",
                "startsAt": "2026-03-20T10:00:00Z",
                "annotations": {
                    "summary": "CPU high"
                }
            }
        ])))
        .mount(&mock_server)
        .await;

    let config = AlertmanagerConfig {
        url: mock_server.uri(),
    };
    let client = reqwest::Client::new();
    let cache = ResponseCache::new(100, Duration::from_secs(60));

    let result = tools::alerts::execute(None, None, &client, &cache, &config).await;

    assert!(result.is_ok(), "alerts::execute() failed: {result:?}");

    let json_str = result.unwrap();
    assert!(
        json_str.contains("HighCPU"),
        "response should contain 'HighCPU', got: {json_str}"
    );
}

#[tokio::test]
async fn test_prometheus_cache_hit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "data": {
                "resultType": "vector",
                "result": []
            }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = PrometheusConfig {
        url: mock_server.uri(),
        auth_token: None,
    };
    let client = reqwest::Client::new();
    let cache = ResponseCache::new(100, Duration::from_secs(60));

    let first =
        tools::prometheus::execute("up", None, None, None, None, &client, &cache, &config).await;
    assert!(first.is_ok(), "first call failed: {first:?}");

    let second =
        tools::prometheus::execute("up", None, None, None, None, &client, &cache, &config).await;
    assert!(second.is_ok(), "second call failed: {second:?}");

    // The mock expectation of exactly 1 request will fail on drop if the
    // cache did not serve the second call.
}
