//! Shared helpers for `OpenF1` API interactions.
//!
//! Provides session key resolution, driver number lookup, and the HTTP fetch
//! helper used by all `OpenF1`-backed tools.

use mcp_common::{McpServerError, ResponseCache};

use crate::config::OpenF1Config;

/// Resolve `(year, grand_prix, session_name)` to an `OpenF1` `session_key`.
///
/// Queries `GET /sessions?year={year}&country_name={gp}&session_name={session}`
/// and returns the first matching `session_key`.
///
/// # Errors
///
/// Returns an error if the `OpenF1` request fails, if no session matches the
/// given parameters, or if `session_key` is not a valid `u64`.
pub async fn resolve_session_key(
    year: u16,
    grand_prix: &str,
    session_name: &str,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &OpenF1Config,
) -> Result<u64, String> {
    let cache_key = format!("session_key:{year}:{grand_prix}:{session_name}");

    let value = cache
        .get_or_fetch(&cache_key, || async {
            let url = format!(
                "{}/sessions?year={}&country_name={}&session_name={}",
                config.base_url, year, grand_prix, session_name,
            );

            let resp = openf1_get(http, &url, config).await?;

            let sessions: Vec<serde_json::Value> =
                serde_json::from_value(resp).map_err(|e| McpServerError::ExternalApi {
                    url: url.clone(),
                    reason: format!("failed to parse sessions array: {e}"),
                })?;

            let key = sessions
                .first()
                .and_then(|s| s.get("session_key"))
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| McpServerError::ExternalApi {
                    url: url.clone(),
                    reason: format!(
                        "no session found for year={year}, gp={grand_prix}, session={session_name}"
                    ),
                })?;

            Ok(serde_json::json!(key))
        })
        .await
        .map_err(|e| e.to_string())?;

    value
        .as_u64()
        .ok_or_else(|| "session_key is not a valid u64".to_string())
}

/// Resolve a driver identifier (name, acronym, or number string) to a
/// `driver_number` within a given session.
///
/// Queries `GET /drivers?session_key={key}` and matches against
/// `name_acronym`, `full_name`, `first_name`, `last_name`, or the
/// number itself if the input parses as an integer.
///
/// # Errors
///
/// Returns an error if the `OpenF1` request fails, if the drivers response is
/// not an array, if no driver matches, or if the number is out of `u16` range.
pub async fn resolve_driver_number(
    session_key: u64,
    driver: &str,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &OpenF1Config,
) -> Result<u16, String> {
    // If the input is already a numeric driver number, return it directly.
    if let Ok(num) = driver.parse::<u16>() {
        return Ok(num);
    }

    let cache_key = format!("drivers:{session_key}");

    let value = cache
        .get_or_fetch(&cache_key, || async {
            let url = format!("{}/drivers?session_key={session_key}", config.base_url,);
            openf1_get(http, &url, config).await
        })
        .await
        .map_err(|e| e.to_string())?;

    let drivers = value
        .as_array()
        .ok_or_else(|| "drivers response is not an array".to_string())?;

    let needle = driver.to_lowercase();

    for d in drivers {
        let matches_acronym = d
            .get("name_acronym")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|s| s.to_lowercase() == needle);

        let matches_full = d
            .get("full_name")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|s| s.to_lowercase().contains(&needle));

        let matches_first = d
            .get("first_name")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|s| s.to_lowercase() == needle);

        let matches_last = d
            .get("last_name")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|s| s.to_lowercase() == needle);

        if (matches_acronym || matches_full || matches_first || matches_last)
            && let Some(num) = d.get("driver_number").and_then(serde_json::Value::as_u64)
        {
            return u16::try_from(num).map_err(|_| format!("driver_number {num} out of u16 range"));
        }
    }

    Err(format!(
        "no driver matching '{driver}' found in session {session_key}"
    ))
}

/// Perform a GET request against the `OpenF1` API, returning the parsed JSON.
///
/// Adds the optional auth token if configured.
///
/// # Errors
///
/// Returns an error if the request fails, if the API returns a non-success
/// status, or if the response body cannot be parsed as JSON.
pub async fn openf1_get(
    http: &reqwest::Client,
    url: &str,
    config: &OpenF1Config,
) -> Result<serde_json::Value, McpServerError> {
    let mut req = http.get(url);
    if let Some(token) = &config.auth_token {
        req = req.bearer_auth(token);
    }

    let resp = req.send().await.map_err(|e| McpServerError::ExternalApi {
        url: url.to_owned(),
        reason: e.to_string(),
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(McpServerError::ExternalApi {
            url: url.to_owned(),
            reason: format!("HTTP {status}: {body}"),
        });
    }

    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| McpServerError::ExternalApi {
            url: url.to_owned(),
            reason: format!("JSON parse error: {e}"),
        })
}
