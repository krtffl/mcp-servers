//! `query_catastro` tool — query the Spanish property registry via Catastro OVC.
//!
//! Queries the Catastro SOAP/XML endpoint using either a cadastral reference
//! or a street address (province + municipality + street + number). Parses
//! the XML response and returns structured JSON.

use mcp_common::ResponseCache;
use serde::Serialize;

use crate::config::CatastroConfig;

#[derive(Debug, Serialize)]
pub struct CatastroProperty {
    pub reference: String,
    pub address: String,
    pub area_m2: Option<f64>,
    pub use_type: String,
    pub construction_year: Option<String>,
}

/// Query Catastro by cadastral reference or address.
///
/// # Errors
///
/// Returns an error if neither a reference nor a complete address is supplied,
/// if the Catastro API request fails or returns a non-success status, or if its
/// response cannot be parsed.
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    reference: Option<&str>,
    province: Option<&str>,
    municipality: Option<&str>,
    street: Option<&str>,
    number: Option<&str>,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &CatastroConfig,
) -> Result<String, String> {
    if reference.is_none() && province.is_none() {
        return Err(
            "Either 'reference' (cadastral reference) or address fields (province, municipality, street) must be provided."
                .to_string(),
        );
    }

    if reference.is_some() {
        query_by_reference(reference.unwrap_or_default(), http, cache, config).await
    } else {
        query_by_address(
            province.unwrap_or_default(),
            municipality.unwrap_or_default(),
            street.unwrap_or_default(),
            number.unwrap_or_default(),
            http,
            cache,
            config,
        )
        .await
    }
}

async fn query_by_reference(
    reference: &str,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &CatastroConfig,
) -> Result<String, String> {
    let cache_key = format!("catastro:ref:{reference}");

    let value = cache
        .get_or_fetch(&cache_key, || async {
            // Catastro OVC REST-like endpoint for reference lookup.
            let url = format!(
                "{}/OVCServWeb/OVCWcfCallejero/COVCCallejero.svc/json/Consulta_DNPRC?RefCat={reference}",
                config.base_url
            );

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

            let properties = parse_catastro_response(&body, reference);

            serde_json::to_value(serde_json::json!({
                "query_type": "reference",
                "reference": reference,
                "results_count": properties.len(),
                "properties": properties,
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

#[allow(clippy::too_many_arguments)]
async fn query_by_address(
    province: &str,
    municipality: &str,
    street: &str,
    number: &str,
    http: &reqwest::Client,
    cache: &ResponseCache,
    config: &CatastroConfig,
) -> Result<String, String> {
    let cache_key = format!("catastro:addr:{province}:{municipality}:{street}:{number}");

    let value = cache
        .get_or_fetch(&cache_key, || async {
            // Catastro OVC REST-like endpoint for address lookup.
            let url = format!(
                "{}/OVCServWeb/OVCWcfCallejero/COVCCallejero.svc/json/Consulta_DNPLOC?Provincia={}&Municipio={}&Sigla=CL&Calle={}&Numero={}",
                config.base_url,
                urlencoded(province),
                urlencoded(municipality),
                urlencoded(street),
                urlencoded(number),
            );

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

            let properties = parse_catastro_address_response(&body);

            serde_json::to_value(serde_json::json!({
                "query_type": "address",
                "province": province,
                "municipality": municipality,
                "street": street,
                "number": number,
                "results_count": properties.len(),
                "properties": properties,
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

/// Parse Catastro JSON response for reference-based queries.
fn parse_catastro_response(body: &serde_json::Value, reference: &str) -> Vec<CatastroProperty> {
    // The Catastro JSON response has varied structure.
    // Try to extract property data from common paths.
    let mut properties = Vec::new();

    // Try path: bico.bi.debi (building detail)
    if let Some(debi) = body.pointer("/bico/bi/debi") {
        let address = body
            .pointer("/bico/bi/ldt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let use_type = debi
            .get("luso")
            .and_then(|v| v.as_str())
            .unwrap_or("Desconocido")
            .to_string();

        let area = debi
            .get("sfc")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok());

        let year = debi.get("ant").and_then(|v| v.as_str()).map(String::from);

        properties.push(CatastroProperty {
            reference: reference.to_string(),
            address,
            area_m2: area,
            use_type,
            construction_year: year,
        });
    }

    // If the above path didn't work, try the list response format.
    if properties.is_empty()
        && let Some(inmuebles) = body
            .pointer("/consulta_dnprcResult/lrcdnp/rcdnp")
            .or_else(|| body.pointer("/lrcdnp/rcdnp"))
    {
        let items = match inmuebles {
            serde_json::Value::Array(arr) => arr.clone(),
            obj @ serde_json::Value::Object(_) => vec![obj.clone()],
            _ => Vec::new(),
        };

        for item in &items {
            let rc = extract_reference(item).unwrap_or_else(|| reference.to_string());
            let address = item
                .pointer("/dt/ldt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            properties.push(CatastroProperty {
                reference: rc,
                address,
                area_m2: None,
                use_type: "Ver detalle con referencia catastral completa".to_string(),
                construction_year: None,
            });
        }
    }

    properties
}

/// Parse Catastro JSON response for address-based queries.
fn parse_catastro_address_response(body: &serde_json::Value) -> Vec<CatastroProperty> {
    let mut properties = Vec::new();

    // Try to navigate the address lookup response structure.
    let inmuebles = body
        .pointer("/consulta_dnplocResult/lrcdnp/rcdnp")
        .or_else(|| body.pointer("/lrcdnp/rcdnp"));

    if let Some(data) = inmuebles {
        let items = match data {
            serde_json::Value::Array(arr) => arr.clone(),
            obj @ serde_json::Value::Object(_) => vec![obj.clone()],
            _ => Vec::new(),
        };

        for item in &items {
            let rc = extract_reference(item).unwrap_or_default();
            let address = item
                .pointer("/dt/ldt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            properties.push(CatastroProperty {
                reference: rc,
                address,
                area_m2: None,
                use_type: "Ver detalle con referencia catastral completa".to_string(),
                construction_year: None,
            });
        }
    }

    properties
}

/// Extract a full cadastral reference from a Catastro response item.
fn extract_reference(item: &serde_json::Value) -> Option<String> {
    let rc = item.get("rc")?;
    let pc1 = rc.get("pc1")?.as_str()?;
    let pc2 = rc.get("pc2")?.as_str()?;
    Some(format!("{pc1}{pc2}"))
}

/// Simple URL encoding for query parameters.
fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('á', "%C3%A1")
        .replace('é', "%C3%A9")
        .replace('í', "%C3%AD")
        .replace('ó', "%C3%B3")
        .replace('ú', "%C3%BA")
        .replace('ñ', "%C3%B1")
        .replace('ü', "%C3%BC")
}
