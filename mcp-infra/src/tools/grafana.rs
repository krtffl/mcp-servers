//! `get_grafana_dashboard_url` tool — resolves dashboard names to URLs.

use mcp_common::ResponseCache;
use serde::Serialize;

use crate::config::GrafanaConfig;

#[derive(Debug, Serialize)]
pub struct GrafanaDashboardUrl {
    pub url: String,
    pub dashboard_title: String,
    pub uid: String,
}

/// Resolve a Grafana dashboard name or UID to its URL.
///
/// # Errors
///
/// Returns an error if the Grafana request fails or returns a non-success
/// status, or if the response cannot be parsed or serialized to JSON.
pub async fn execute(
    name: Option<&str>,
    uid: Option<&str>,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &GrafanaConfig,
) -> Result<String, String> {
    let cache_key = format!("grafana:{}:{}", name.unwrap_or(""), uid.unwrap_or(""));

    let value = cache
        .get_or_fetch(&cache_key, || async {
            let url = format!("{}/api/search", config.url);
            let mut params = vec![("type", "dash-db".to_string())];

            if let Some(q) = name {
                params.push(("query", q.to_string()));
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

            let dashboards: Vec<serde_json::Value> =
                resp.json()
                    .await
                    .map_err(|e| mcp_common::McpServerError::ExternalApi {
                        url: url.clone(),
                        reason: format!("JSON parse error: {e}"),
                    })?;

            // Filter by uid if provided
            let results: Vec<GrafanaDashboardUrl> = dashboards
                .iter()
                .filter(|d| {
                    if let Some(target_uid) = uid {
                        d.get("uid")
                            .and_then(|u| u.as_str())
                            .is_some_and(|u| u == target_uid)
                    } else {
                        true
                    }
                })
                .filter_map(|d| {
                    let d_uid = d.get("uid")?.as_str()?.to_string();
                    let title = d.get("title")?.as_str()?.to_string();
                    let dash_url = format!(
                        "{}/d/{}/{}",
                        config.url,
                        d_uid,
                        title.to_lowercase().replace(' ', "-")
                    );
                    Some(GrafanaDashboardUrl {
                        url: dash_url,
                        dashboard_title: title,
                        uid: d_uid,
                    })
                })
                .collect();

            serde_json::to_value(&results).map_err(|e| mcp_common::McpServerError::ExternalApi {
                url,
                reason: format!("serialization error: {e}"),
            })
        })
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_string_pretty(&value).map_err(|e| format!("JSON error: {e}"))
}
