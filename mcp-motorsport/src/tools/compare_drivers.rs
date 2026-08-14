//! `compare_drivers` tool — head-to-head driver comparison within a session.
//!
//! Fetches lap times for both drivers, computes per-lap deltas, and returns
//! aggregate statistics (average pace, best lap, gap evolution).

use std::collections::HashMap;

use mcp_common::ResponseCache;

use super::common::{openf1_get, resolve_driver_number, resolve_session_key};
use crate::config::OpenF1Config;

/// Compare two drivers' lap times in a session.
///
/// # Errors
///
/// Returns an error if the session or either driver cannot be resolved, if the
/// `OpenF1` request fails, or if the response fails to serialize to JSON.
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    year: u16,
    grand_prix: &str,
    session: &str,
    driver_a: &str,
    driver_b: &str,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &OpenF1Config,
) -> Result<String, String> {
    let session_key = resolve_session_key(year, grand_prix, session, http, cache, config).await?;
    let num_a = resolve_driver_number(session_key, driver_a, http, cache, config).await?;
    let num_b = resolve_driver_number(session_key, driver_b, http, cache, config).await?;

    let cache_key = format!("compare:{session_key}:{num_a}:{num_b}");

    let value = cache
        .get_or_fetch(&cache_key, || async {
            let url_a = format!(
                "{}/laps?session_key={session_key}&driver_number={num_a}",
                config.base_url,
            );
            let url_b = format!(
                "{}/laps?session_key={session_key}&driver_number={num_b}",
                config.base_url,
            );

            let (laps_json_a, laps_json_b) = tokio::try_join!(
                openf1_get(http, &url_a, config),
                openf1_get(http, &url_b, config),
            )?;

            // Index lap times by lap_number.
            let times_a = index_lap_times(&laps_json_a);
            let times_b = index_lap_times(&laps_json_b);

            // Compute per-lap deltas for laps both drivers completed.
            let mut deltas: Vec<serde_json::Value> = Vec::new();
            let mut sum_a: f64 = 0.0;
            let mut sum_b: f64 = 0.0;
            let mut count: u64 = 0;
            let mut best_a: f64 = f64::MAX;
            let mut best_b: f64 = f64::MAX;

            let mut all_laps: Vec<u64> = times_a.keys().copied().collect();
            all_laps.sort_unstable();

            for lap_num in &all_laps {
                if let (Some(&time_a), Some(time_b)) = (times_a.get(lap_num), times_b.get(lap_num))
                {
                    let delta = time_a - time_b; // negative means driver A is faster
                    deltas.push(serde_json::json!({
                        "lap": lap_num,
                        "driver_a_time": time_a,
                        "driver_b_time": time_b,
                        "delta_seconds": (delta * 1000.0).round() / 1000.0,
                    }));
                    sum_a += time_a;
                    sum_b += *time_b;
                    count += 1;
                    if time_a < best_a {
                        best_a = time_a;
                    }
                    if *time_b < best_b {
                        best_b = *time_b;
                    }
                }
            }

            #[allow(clippy::cast_precision_loss)]
            let avg_a = if count > 0 {
                (sum_a / count as f64 * 1000.0).round() / 1000.0
            } else {
                0.0
            };
            #[allow(clippy::cast_precision_loss)]
            let avg_b = if count > 0 {
                (sum_b / count as f64 * 1000.0).round() / 1000.0
            } else {
                0.0
            };

            Ok(serde_json::json!({
                "year": year,
                "grand_prix": grand_prix,
                "session": session,
                "driver_a": driver_a,
                "driver_a_number": num_a,
                "driver_b": driver_b,
                "driver_b_number": num_b,
                "comparable_laps": count,
                "average_pace": {
                    "driver_a": avg_a,
                    "driver_b": avg_b,
                    "delta": ((avg_a - avg_b) * 1000.0).round() / 1000.0,
                },
                "best_lap": {
                    "driver_a": if best_a < f64::MAX { Some(best_a) } else { None },
                    "driver_b": if best_b < f64::MAX { Some(best_b) } else { None },
                },
                "lap_deltas": deltas,
            }))
        })
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_string_pretty(&value).map_err(|e| format!("JSON error: {e}"))
}

/// Build a `HashMap<lap_number, duration_seconds>` from the `OpenF1` laps response.
fn index_lap_times(json: &serde_json::Value) -> HashMap<u64, f64> {
    json.as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|lap| {
            let num = lap.get("lap_number")?.as_u64()?;
            let dur = lap.get("lap_duration")?.as_f64()?;
            Some((num, dur))
        })
        .collect()
}
