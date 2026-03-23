//! `get_telemetry` tool — fetch car telemetry data (speed, throttle, brake, gear, DRS).
//!
//! Queries the `OpenF1` `car_data` endpoint for a given driver in a session.
//! Optionally filters to a specific lap by cross-referencing with lap timing data.
//! Downsamples to ~100 data points per lap for manageable output.

use std::fmt::Write as _;

use mcp_common::ResponseCache;

use crate::config::OpenF1Config;
use super::common::{openf1_get, resolve_driver_number, resolve_session_key};

/// Maximum telemetry samples to return (keeps output manageable for LLM consumption).
const MAX_SAMPLES: usize = 100;

/// Fetch telemetry data for a driver in a session, optionally for a specific lap.
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    year: u16,
    grand_prix: &str,
    session: &str,
    driver: &str,
    lap: Option<u16>,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &OpenF1Config,
) -> Result<String, String> {
    let session_key =
        resolve_session_key(year, grand_prix, session, http, cache, config).await?;
    let driver_number =
        resolve_driver_number(session_key, driver, http, cache, config).await?;

    let cache_key = format!(
        "telemetry:{session_key}:{driver_number}:{}",
        lap.map_or("all".to_string(), |l| l.to_string()),
    );

    let value = cache
        .get_or_fetch(&cache_key, || async {
            // If a specific lap is requested, fetch lap data first to get time bounds.
            #[allow(clippy::similar_names)]
            let (date_start, date_end) = if let Some(lap_num) = lap {
                let laps_url = format!(
                    "{}/laps?session_key={session_key}&driver_number={driver_number}&lap_number={lap_num}",
                    config.base_url,
                );
                let laps_json = openf1_get(http, &laps_url, config).await?;
                let laps_arr = laps_json.as_array();

                if let Some(lap_entry) = laps_arr.and_then(|a| a.first()) {
                    let start = lap_entry
                        .get("date_start")
                        .and_then(serde_json::Value::as_str)
                        .map(String::from);

                    // Compute end timestamp from start + duration using chrono.
                    let duration = lap_entry
                        .get("lap_duration")
                        .and_then(serde_json::Value::as_f64);

                    let end = match (&start, duration) {
                        (Some(s), Some(dur)) => {
                            if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(s) {
                                #[allow(clippy::cast_possible_truncation)]
                                let millis = (dur * 1000.0) as i64;
                                let end_time = parsed + chrono::Duration::milliseconds(millis);
                                Some(end_time.to_rfc3339())
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };

                    (start, end)
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            // Fetch car data, applying time bounds if available.
            let mut url = format!(
                "{}/car_data?session_key={session_key}&driver_number={driver_number}",
                config.base_url,
            );
            if let Some(ref gt) = date_start {
                let _ = write!(url, "&date>{gt}");
            }
            if let Some(ref lt) = date_end {
                let _ = write!(url, "&date<{lt}");
            }

            let car_data_json = openf1_get(http, &url, config).await?;

            let all_samples = car_data_json.as_array().cloned().unwrap_or_default();

            // Downsample to MAX_SAMPLES evenly spaced points.
            let samples = downsample(&all_samples, MAX_SAMPLES);

            let telemetry: Vec<serde_json::Value> = samples
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "date": s.get("date").and_then(serde_json::Value::as_str).unwrap_or(""),
                        "speed": s.get("speed").and_then(serde_json::Value::as_u64).unwrap_or(0),
                        "throttle": s.get("throttle").and_then(serde_json::Value::as_u64).unwrap_or(0),
                        "brake": s.get("brake").and_then(serde_json::Value::as_u64).unwrap_or(0),
                        "gear": s.get("n_gear").and_then(serde_json::Value::as_u64).unwrap_or(0),
                        "drs": s.get("drs").and_then(serde_json::Value::as_u64).unwrap_or(0),
                        "rpm": s.get("rpm").and_then(serde_json::Value::as_u64),
                    })
                })
                .collect();

            Ok(serde_json::json!({
                "year": year,
                "grand_prix": grand_prix,
                "session": session,
                "driver": driver,
                "driver_number": driver_number,
                "lap": lap,
                "total_raw_samples": all_samples.len(),
                "samples_returned": telemetry.len(),
                "telemetry": telemetry,
            }))
        })
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_string_pretty(&value).map_err(|e| format!("JSON error: {e}"))
}

/// Downsample a slice to at most `max` evenly spaced elements.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn downsample(data: &[serde_json::Value], max: usize) -> Vec<serde_json::Value> {
    let len = data.len();
    if len <= max {
        return data.to_vec();
    }

    let step = len as f64 / max as f64;
    (0..max)
        .map(|i| {
            let idx = (i as f64 * step).floor() as usize;
            data[idx.min(len - 1)].clone()
        })
        .collect()
}
