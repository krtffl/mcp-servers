//! `get_standings` tool — fetch driver or constructor championship standings.
//!
//! Uses the Jolpica-F1 API (Ergast replacement) instead of `OpenF1`, as `OpenF1`
//! does not provide cumulative standings data.
//!
//! Jolpica endpoints:
//! - `GET /ergast/f1/{year}/driverStandings.json`
//! - `GET /ergast/f1/{year}/constructorStandings.json`

use mcp_common::ResponseCache;

use crate::config::JolpicaConfig;

/// Fetch driver or constructor championship standings for a season.
pub async fn execute(
    year: u16,
    standings_type: &str,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &JolpicaConfig,
) -> Result<String, String> {
    let st = match standings_type.to_lowercase().as_str() {
        "drivers" | "driver" => "driverStandings",
        "constructors" | "constructor" | "teams" | "team" => "constructorStandings",
        other => return Err(format!(
            "Invalid standings_type '{other}'. Use 'drivers' or 'constructors'."
        )),
    };

    let cache_key = format!("standings:{year}:{st}");

    let value = cache
        .get_or_fetch(&cache_key, || async {
            let url = format!("{}/{year}/{st}.json", config.base_url);

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

            let body: serde_json::Value = resp.json().await.map_err(|e| {
                mcp_common::McpServerError::ExternalApi {
                    url: url.clone(),
                    reason: format!("JSON parse error: {e}"),
                }
            })?;

            let standings = if st == "driverStandings" {
                parse_driver_standings(&body)
            } else {
                parse_constructor_standings(&body)
            };

            Ok(serde_json::json!({
                "year": year,
                "standings_type": standings_type,
                "standings_count": standings.len(),
                "standings": standings,
            }))
        })
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_string_pretty(&value).map_err(|e| format!("JSON error: {e}"))
}

/// Parse the Jolpica/Ergast driver standings response.
fn parse_driver_standings(body: &serde_json::Value) -> Vec<serde_json::Value> {
    let standings_list = body
        .pointer("/MRData/StandingsTable/StandingsLists")
        .and_then(serde_json::Value::as_array);

    let Some(lists) = standings_list else {
        return Vec::new();
    };

    let entries = lists
        .first()
        .and_then(|l| l.get("DriverStandings"))
        .and_then(serde_json::Value::as_array);

    let Some(standings) = entries else {
        return Vec::new();
    };

    standings
        .iter()
        .filter_map(|entry| {
            let position = entry.get("position")?.as_str()?;
            let points = entry.get("points")?.as_str()?;
            let wins = entry.get("wins")?.as_str()?;

            let driver = entry.get("Driver")?;
            let first = driver.get("givenName")?.as_str().unwrap_or("");
            let last = driver.get("familyName")?.as_str().unwrap_or("");
            let code = driver
                .get("code")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");

            let team = entry
                .get("Constructors")
                .and_then(serde_json::Value::as_array)
                .and_then(|c| c.first())
                .and_then(|c| c.get("name"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Unknown");

            Some(serde_json::json!({
                "position": position,
                "driver": format!("{first} {last}"),
                "driver_code": code,
                "team": team,
                "points": points,
                "wins": wins,
            }))
        })
        .collect()
}

/// Parse the Jolpica/Ergast constructor standings response.
fn parse_constructor_standings(body: &serde_json::Value) -> Vec<serde_json::Value> {
    let standings_list = body
        .pointer("/MRData/StandingsTable/StandingsLists")
        .and_then(serde_json::Value::as_array);

    let Some(lists) = standings_list else {
        return Vec::new();
    };

    let entries = lists
        .first()
        .and_then(|l| l.get("ConstructorStandings"))
        .and_then(serde_json::Value::as_array);

    let Some(standings) = entries else {
        return Vec::new();
    };

    standings
        .iter()
        .filter_map(|entry| {
            let position = entry.get("position")?.as_str()?;
            let points = entry.get("points")?.as_str()?;
            let wins = entry.get("wins")?.as_str()?;

            let constructor = entry.get("Constructor")?;
            let name = constructor.get("name")?.as_str()?;
            let nationality = constructor
                .get("nationality")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");

            Some(serde_json::json!({
                "position": position,
                "constructor": name,
                "nationality": nationality,
                "points": points,
                "wins": wins,
            }))
        })
        .collect()
}
