//! `query_prometheus` tool — `PromQL` queries against Prometheus HTTP API.

use crate::config::PrometheusConfig;
use mcp_common::ResponseCache;

/// Run an instant or range `PromQL` query against the Prometheus HTTP API.
///
/// # Errors
///
/// Returns an error if the Prometheus request fails or returns a non-success
/// status, or if the response cannot be parsed or serialized to JSON.
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    query: &str,
    time: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
    step: Option<&str>,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &PrometheusConfig,
) -> Result<String, String> {
    let is_range = start.is_some() && end.is_some();
    let cache_key = format!(
        "prometheus:{}:{}:{}:{}:{}",
        query,
        time.unwrap_or(""),
        start.unwrap_or(""),
        end.unwrap_or(""),
        step.unwrap_or("")
    );

    let value = cache
        .get_or_fetch(&cache_key, || async {
            let endpoint = if is_range { "query_range" } else { "query" };
            let url = format!("{}/api/v1/{endpoint}", config.url);

            let mut params = vec![("query", query.to_string())];
            if let Some(t) = time {
                params.push(("time", t.to_string()));
            }
            if let Some(s) = start {
                params.push(("start", s.to_string()));
            }
            if let Some(e) = end {
                params.push(("end", e.to_string()));
            }
            if let Some(s) = step {
                params.push(("step", s.to_string()));
            }

            let mut req = http.get(&url).query(&params);
            if let Some(token) = &config.auth_token {
                req = req.bearer_auth(token);
            }

            let resp = req
                .send()
                .await
                .map_err(|e| mcp_common::McpServerError::ExternalApi {
                    url: url.clone(),
                    reason: e.to_string(),
                })?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(mcp_common::McpServerError::ExternalApi {
                    url,
                    reason: format!("HTTP {status}: {body}"),
                });
            }

            resp.json::<serde_json::Value>().await.map_err(|e| {
                mcp_common::McpServerError::ExternalApi {
                    url,
                    reason: format!("JSON parse error: {e}"),
                }
            })
        })
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_string_pretty(&value).map_err(|e| format!("JSON error: {e}"))
}
