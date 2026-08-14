//! `get_lap_times` tool — fetch per-lap timing data for a driver.
//!
//! Resolves the session key and driver number, then queries the `OpenF1`
//! laps endpoint to return lap-by-lap durations.

use mcp_common::ResponseCache;

use super::common::{openf1_get, resolve_driver_number, resolve_session_key};
use crate::config::OpenF1Config;

/// Fetch lap times for a specific driver in a session.
///
/// # Errors
///
/// Returns an error if the session or driver cannot be resolved, if the
/// `OpenF1` request fails, or if the response fails to serialize to JSON.
pub async fn execute(
    year: u16,
    grand_prix: &str,
    session: &str,
    driver: &str,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &OpenF1Config,
) -> Result<String, String> {
    let session_key = resolve_session_key(year, grand_prix, session, http, cache, config).await?;
    let driver_number = resolve_driver_number(session_key, driver, http, cache, config).await?;

    let cache_key = format!("lap_times:{session_key}:{driver_number}");

    let value = cache
        .get_or_fetch(&cache_key, || async {
            let url = format!(
                "{}/laps?session_key={session_key}&driver_number={driver_number}",
                config.base_url,
            );

            let laps_json = openf1_get(http, &url, config).await?;

            let laps: Vec<serde_json::Value> = laps_json
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|lap| {
                    let lap_number = lap.get("lap_number")?.as_u64()?;
                    let duration = lap.get("lap_duration").and_then(serde_json::Value::as_f64);
                    let s1 = lap
                        .get("duration_sector_1")
                        .and_then(serde_json::Value::as_f64);
                    let s2 = lap
                        .get("duration_sector_2")
                        .and_then(serde_json::Value::as_f64);
                    let s3 = lap
                        .get("duration_sector_3")
                        .and_then(serde_json::Value::as_f64);
                    let is_pit_out = lap
                        .get("is_pit_out_lap")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false);

                    Some(serde_json::json!({
                        "lap_number": lap_number,
                        "duration_seconds": duration,
                        "sector_1": s1,
                        "sector_2": s2,
                        "sector_3": s3,
                        "is_pit_out_lap": is_pit_out,
                    }))
                })
                .collect();

            Ok(serde_json::json!({
                "year": year,
                "grand_prix": grand_prix,
                "session": session,
                "driver": driver,
                "driver_number": driver_number,
                "laps_count": laps.len(),
                "laps": laps,
            }))
        })
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_string_pretty(&value).map_err(|e| format!("JSON error: {e}"))
}
