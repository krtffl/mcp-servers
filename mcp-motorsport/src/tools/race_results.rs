//! `get_race_results` tool — fetch final race classification.
//!
//! Resolves the race session key, fetches position data and driver info,
//! then assembles a final results table sorted by finishing position.

use std::collections::HashMap;

use mcp_common::ResponseCache;

use crate::config::OpenF1Config;
use super::common::{openf1_get, resolve_session_key};

/// Fetch race results for a given Grand Prix.
pub async fn execute(
    year: u16,
    grand_prix: &str,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &OpenF1Config,
) -> Result<String, String> {
    let cache_key = format!("race_results:{year}:{grand_prix}");

    let value = cache
        .get_or_fetch(&cache_key, || async {
            let session_key =
                resolve_session_key(year, grand_prix, "Race", http, cache, config).await
                    .map_err(|e| mcp_common::McpServerError::ExternalApi {
                        url: String::new(),
                        reason: e,
                    })?;

            // Fetch positions and drivers in parallel.
            let positions_url = format!(
                "{}/position?session_key={session_key}",
                config.base_url,
            );
            let drivers_url = format!(
                "{}/drivers?session_key={session_key}",
                config.base_url,
            );

            let (positions_json, drivers_json) = tokio::try_join!(
                openf1_get(http, &positions_url, config),
                openf1_get(http, &drivers_url, config),
            )?;

            // Build a driver_number -> driver info map.
            let drivers: HashMap<u64, serde_json::Value> = drivers_json
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|d| {
                    let num = d.get("driver_number")?.as_u64()?;
                    Some((num, d.clone()))
                })
                .collect();

            // Keep the last (final) position entry per driver.
            let mut final_positions: HashMap<u64, u64> = HashMap::new();
            if let Some(positions) = positions_json.as_array() {
                for p in positions {
                    let driver_num = p.get("driver_number").and_then(serde_json::Value::as_u64);
                    let position = p.get("position").and_then(serde_json::Value::as_u64);
                    if let (Some(num), Some(pos)) = (driver_num, position) {
                        final_positions.insert(num, pos);
                    }
                }
            }

            // Assemble results sorted by position.
            let mut results: Vec<serde_json::Value> = final_positions
                .iter()
                .map(|(&driver_num, &position)| {
                    let driver_info = drivers.get(&driver_num);
                    let name = driver_info
                        .and_then(|d| d.get("full_name"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("Unknown");
                    let team = driver_info
                        .and_then(|d| d.get("team_name"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("Unknown");
                    let acronym = driver_info
                        .and_then(|d| d.get("name_acronym"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");

                    serde_json::json!({
                        "position": position,
                        "driver_number": driver_num,
                        "driver_name": name,
                        "driver_acronym": acronym,
                        "team": team,
                    })
                })
                .collect();

            results.sort_by_key(|r| r.get("position").and_then(serde_json::Value::as_u64).unwrap_or(999));

            Ok(serde_json::json!({
                "year": year,
                "grand_prix": grand_prix,
                "session": "Race",
                "results_count": results.len(),
                "results": results,
            }))
        })
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_string_pretty(&value).map_err(|e| format!("JSON error: {e}"))
}
