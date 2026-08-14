//! `get_tire_stints` tool — fetch tire stint data for a race session.
//!
//! Queries the `OpenF1` stints endpoint to return compound, stint number,
//! and lap ranges per driver. Optionally filters to a single driver.

use std::collections::HashMap;

use mcp_common::ResponseCache;

use super::common::{openf1_get, resolve_driver_number, resolve_session_key};
use crate::config::OpenF1Config;

/// Fetch tire stint data for a Grand Prix race, optionally filtered by driver.
///
/// # Errors
///
/// Returns an error if the session or driver cannot be resolved, if the
/// `OpenF1` request fails, or if the response fails to serialize to JSON.
pub async fn execute(
    year: u16,
    grand_prix: &str,
    driver: Option<&str>,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &OpenF1Config,
) -> Result<String, String> {
    let session_key = resolve_session_key(year, grand_prix, "Race", http, cache, config).await?;

    let driver_number = if let Some(d) = driver {
        Some(resolve_driver_number(session_key, d, http, cache, config).await?)
    } else {
        None
    };

    let cache_key = format!(
        "tire_stints:{session_key}:{}",
        driver_number.map_or("all".to_string(), |n| n.to_string()),
    );

    let value = cache
        .get_or_fetch(&cache_key, || async {
            let url = if let Some(num) = driver_number {
                format!(
                    "{}/stints?session_key={session_key}&driver_number={num}",
                    config.base_url,
                )
            } else {
                format!("{}/stints?session_key={session_key}", config.base_url,)
            };

            let stints_json = openf1_get(http, &url, config).await?;

            // Fetch driver names for display.
            let drivers_url = format!("{}/drivers?session_key={session_key}", config.base_url,);
            let drivers_json = openf1_get(http, &drivers_url, config).await?;

            let driver_names: HashMap<u64, String> = drivers_json
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|d| {
                    let num = d.get("driver_number")?.as_u64()?;
                    let name = d.get("full_name")?.as_str()?.to_string();
                    Some((num, name))
                })
                .collect();

            let stints: Vec<serde_json::Value> = stints_json
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|stint| {
                    let num = stint.get("driver_number")?.as_u64()?;
                    let stint_number = stint.get("stint_number")?.as_u64()?;
                    let compound = stint
                        .get("compound")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("UNKNOWN");
                    let lap_start = stint.get("lap_start").and_then(serde_json::Value::as_u64)?;
                    let lap_end = stint.get("lap_end").and_then(serde_json::Value::as_u64)?;
                    let tyre_age = stint
                        .get("tyre_age_at_start")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let driver_name = driver_names.get(&num).map_or("Unknown", String::as_str);

                    Some(serde_json::json!({
                        "driver_number": num,
                        "driver_name": driver_name,
                        "stint_number": stint_number,
                        "compound": compound,
                        "lap_start": lap_start,
                        "lap_end": lap_end,
                        "laps": lap_end - lap_start + 1,
                        "tyre_age_at_start": tyre_age,
                    }))
                })
                .collect();

            Ok(serde_json::json!({
                "year": year,
                "grand_prix": grand_prix,
                "session": "Race",
                "driver_filter": driver,
                "stints_count": stints.len(),
                "stints": stints,
            }))
        })
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_string_pretty(&value).map_err(|e| format!("JSON error: {e}"))
}
