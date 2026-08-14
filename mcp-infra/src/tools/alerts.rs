//! `list_recent_alerts` tool — Alertmanager alert listing.

use mcp_common::ResponseCache;
use serde::Serialize;

use crate::config::AlertmanagerConfig;

#[derive(Debug, Serialize)]
pub struct AlertInfo {
    pub name: String,
    pub status: String,
    pub severity: String,
    pub started_at: String,
    pub annotations: std::collections::HashMap<String, String>,
}

/// List recent Alertmanager alerts, optionally filtered by status and severity.
///
/// # Errors
///
/// Returns an error if the Alertmanager request fails or returns a non-success
/// status, or if the response cannot be parsed or serialized to JSON.
pub async fn execute(
    status_filter: Option<&str>,
    severity_filter: Option<&str>,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &AlertmanagerConfig,
) -> Result<String, String> {
    let cache_key = format!(
        "alerts:{}:{}",
        status_filter.unwrap_or(""),
        severity_filter.unwrap_or("")
    );

    let value = cache
        .get_or_fetch(&cache_key, || async {
            let url = format!("{}/api/v2/alerts", config.url);

            let mut params = Vec::new();
            if let Some(s) = status_filter {
                params.push(("filter", format!("status=\"{s}\"")));
            }

            let resp = http.get(&url).query(&params).send().await.map_err(|e| {
                mcp_common::McpServerError::ExternalApi {
                    url: url.clone(),
                    reason: e.to_string(),
                }
            })?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(mcp_common::McpServerError::ExternalApi {
                    url,
                    reason: format!("HTTP {status}: {body}"),
                });
            }

            let raw_alerts: Vec<serde_json::Value> =
                resp.json()
                    .await
                    .map_err(|e| mcp_common::McpServerError::ExternalApi {
                        url: url.clone(),
                        reason: format!("JSON parse error: {e}"),
                    })?;

            let alerts: Vec<AlertInfo> = raw_alerts
                .iter()
                .filter(|a| {
                    if let Some(sev) = severity_filter {
                        a.pointer("/labels/severity")
                            .and_then(|s| s.as_str())
                            .is_some_and(|s| s == sev)
                    } else {
                        true
                    }
                })
                .map(|a| {
                    let labels = a.get("labels").cloned().unwrap_or_default();
                    let annotations_val = a.get("annotations").cloned().unwrap_or_default();
                    let annotations: std::collections::HashMap<String, String> = annotations_val
                        .as_object()
                        .map(|obj| {
                            obj.iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    AlertInfo {
                        name: labels
                            .get("alertname")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        status: a
                            .get("status")
                            .and_then(|s| s.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        severity: labels
                            .get("severity")
                            .and_then(|s| s.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        started_at: a
                            .get("startsAt")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string(),
                        annotations,
                    }
                })
                .collect();

            serde_json::to_value(&alerts).map_err(|e| mcp_common::McpServerError::ExternalApi {
                url,
                reason: format!("serialization error: {e}"),
            })
        })
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_string_pretty(&value).map_err(|e| format!("JSON error: {e}"))
}
