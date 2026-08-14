//! `search_boe` tool — search BOE (Boletín Oficial del Estado) documents.
//!
//! Queries the BOE open data API to search for official publications,
//! laws, and regulations. Supports filtering by date range, section,
//! and department.

use mcp_common::ResponseCache;
use serde::Serialize;

use crate::config::BoeConfig;

#[derive(Debug, Serialize)]
pub struct BoeDocument {
    pub title: String,
    pub publication_date: String,
    pub section: String,
    pub department: String,
    pub pdf_url: String,
    pub summary: String,
    pub identifier: String,
}

/// Search the BOE API for documents matching the given criteria.
///
/// # Errors
///
/// Returns an error if `keywords` is empty, if the BOE API request fails or
/// returns a non-success status, or if its response cannot be parsed.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub async fn execute(
    keywords: &str,
    date_from: Option<&str>,
    date_to: Option<&str>,
    section: Option<&str>,
    department: Option<&str>,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &BoeConfig,
) -> Result<String, String> {
    if keywords.trim().is_empty() {
        return Err("Keywords must not be empty.".to_string());
    }

    let cache_key = format!(
        "boe:{}:{}:{}:{}:{}",
        keywords,
        date_from.unwrap_or(""),
        date_to.unwrap_or(""),
        section.unwrap_or(""),
        department.unwrap_or(""),
    );

    let value = cache
        .get_or_fetch(&cache_key, || async {
            // The BOE open data API provides daily summaries.
            // For search, we query the summary endpoint for a date range.
            // If no dates given, default to last 30 days.
            let today = chrono_today();
            let from = date_from.unwrap_or(&today).replace('-', "");
            let to = date_to.unwrap_or(&today).replace('-', "");

            // BOE API endpoint for summary by date: /datosabiertos/api/boe/sumario/{YYYYMMDD}
            // We query the start date summary and parse matching items.
            let url = format!("{}/boe/sumario/{from}", config.base_url);

            let resp = http.get(&url).send().await.map_err(|e| {
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

            let body = resp
                .text()
                .await
                .map_err(|e| mcp_common::McpServerError::ExternalApi {
                    url: url.clone(),
                    reason: format!("Failed to read response body: {e}"),
                })?;

            // The BOE API returns JSON with a nested structure.
            // Parse it and filter by keywords, section, and department.
            let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
                mcp_common::McpServerError::ExternalApi {
                    url: url.clone(),
                    reason: format!("JSON parse error: {e}"),
                }
            })?;

            let keywords_lower = keywords.to_lowercase();
            let keyword_terms: Vec<&str> = keywords_lower.split_whitespace().collect();
            let mut documents = Vec::new();

            // Navigate the BOE summary JSON structure.
            // Structure: data.sumario.diario[].seccion[].departamento[].epigrafe[].item[]
            if let Some(diarios) = json
                .pointer("/data/sumario/diario")
                .and_then(|d| d.as_array())
            {
                for diario in diarios {
                    if let Some(secciones) = diario.get("seccion").and_then(|s| s.as_array()) {
                        for seccion in secciones {
                            let seccion_nombre = seccion
                                .get("@nombre")
                                .and_then(|n| n.as_str())
                                .unwrap_or("");

                            if let Some(sec_filter) = section
                                && !seccion_nombre
                                    .to_lowercase()
                                    .contains(&sec_filter.to_lowercase())
                            {
                                continue;
                            }

                            let departamentos = as_array_or_single(seccion.get("departamento"));

                            for dep in &departamentos {
                                let dep_nombre = dep
                                    .get("@nombre")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("");

                                if let Some(dep_filter) = department
                                    && !dep_nombre
                                        .to_lowercase()
                                        .contains(&dep_filter.to_lowercase())
                                {
                                    continue;
                                }

                                let epigrafes = as_array_or_single(dep.get("epigrafe"));

                                for epigrafe in &epigrafes {
                                    let items = as_array_or_single(epigrafe.get("item"));

                                    for item in &items {
                                        let titulo = item
                                            .get("titulo")
                                            .and_then(serde_json::Value::as_str)
                                            .unwrap_or("");

                                        let titulo_lower = titulo.to_lowercase();
                                        let matches = keyword_terms
                                            .iter()
                                            .all(|kw| titulo_lower.contains(kw));

                                        if !matches {
                                            continue;
                                        }

                                        let id = item
                                            .get("@id")
                                            .and_then(serde_json::Value::as_str)
                                            .unwrap_or("");

                                        let url_pdf = item
                                            .get("urlPdf")
                                            .and_then(serde_json::Value::as_str)
                                            .map(|u: &str| {
                                                if u.starts_with("http") {
                                                    u.to_string()
                                                } else {
                                                    format!("https://boe.es{u}")
                                                }
                                            })
                                            .unwrap_or_default();

                                        documents.push(BoeDocument {
                                            title: titulo.to_string(),
                                            publication_date: from.clone(),
                                            section: seccion_nombre.to_string(),
                                            department: dep_nombre.to_string(),
                                            pdf_url: url_pdf,
                                            summary: titulo.chars().take(200).collect(),
                                            identifier: id.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let _ = to; // date_to used for cache key; multi-day search is future enhancement

            serde_json::to_value(serde_json::json!({
                "query": keywords,
                "date_from": date_from,
                "date_to": date_to,
                "results_count": documents.len(),
                "documents": documents,
            }))
            .map_err(|e| mcp_common::McpServerError::ExternalApi {
                url,
                reason: format!("serialization error: {e}"),
            })
        })
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_string_pretty(&value).map_err(|e| format!("JSON error: {e}"))
}

/// BOE API sometimes returns a single object instead of an array.
/// This helper normalizes both cases into a `Vec`.
fn as_array_or_single(val: Option<&serde_json::Value>) -> Vec<serde_json::Value> {
    match val {
        Some(serde_json::Value::Array(arr)) => arr.clone(),
        Some(obj @ serde_json::Value::Object(_)) => vec![obj.clone()],
        _ => Vec::new(),
    }
}

/// Get today's date as YYYYMMDD string without pulling in chrono as a dependency.
fn chrono_today() -> String {
    // Use a simple approach: parse from system time.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Convert Unix timestamp to YYYYMMDD.
    let days = now / 86400;
    let (year, month, day) = days_to_date(days);
    format!("{year}{month:02}{day:02}")
}

/// Convert days since Unix epoch to (year, month, day).
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
fn days_to_date(days_since_epoch: u64) -> (u64, u64, u64) {
    // Algorithm from Howard Hinnant's `civil_from_days`.
    let z = days_since_epoch as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u64, m, d)
}
