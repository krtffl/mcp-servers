//! Integration tests for mcp-spain tools.

use std::time::Duration;

use mcp_common::ResponseCache;
use mcp_spain::config::BoeConfig;
use mcp_spain::tools;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// CNAE tests (static embedded data, no HTTP)
// ---------------------------------------------------------------------------

#[test]
fn test_cnae_lookup_by_code() {
    let result = tools::cnae::execute(Some("6201"), None);
    let json = result.expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert!(parsed["matches"].as_u64().unwrap_or(0) > 0);

    let first_desc = parsed["results"][0]["description"]
        .as_str()
        .expect("description should be a string")
        .to_lowercase();
    assert!(
        first_desc.contains("programación informática"),
        "expected CNAE 6201 to mention programming, got: {first_desc}",
    );
}

#[test]
fn test_cnae_lookup_by_description() {
    let result = tools::cnae::execute(None, Some("inmobiliaria"));
    let json = result.expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert!(
        parsed["matches"].as_u64().unwrap_or(0) > 0,
        "expected at least one match for 'inmobiliaria'",
    );
}

#[test]
fn test_cnae_no_match() {
    let result = tools::cnae::execute(Some("9999"), None);
    let json = result.expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert_eq!(
        parsed["matches"].as_u64().unwrap_or(1),
        0,
        "expected no matches for code 9999",
    );
    assert!(
        parsed["results"].as_array().unwrap_or(&vec![]).is_empty(),
        "expected empty results array",
    );
}

// ---------------------------------------------------------------------------
// AEAT calendar tests (static embedded data, no HTTP)
// ---------------------------------------------------------------------------

#[test]
fn test_aeat_calendar_q1() {
    let result = tools::aeat::execute(2026, Some(1), None);
    let json = result.expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert_eq!(parsed["year"].as_u64(), Some(2026));
    assert_eq!(parsed["quarter_filter"].as_u64(), Some(1));

    let count = parsed["deadlines_count"].as_u64().unwrap_or(0);
    assert!(count > 0, "expected Q1 deadlines, got 0");

    // Q1 should contain model 303 (IVA)
    let deadlines = parsed["deadlines"].as_array().expect("deadlines array");
    let has_303 = deadlines.iter().any(|d| d["model"].as_str() == Some("303"));
    assert!(has_303, "Q1 should include model 303 (IVA)");
}

#[test]
fn test_aeat_calendar_filter_by_business_type() {
    let result = tools::aeat::execute(2026, None, Some("autonomo"));
    let json = result.expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert_eq!(parsed["business_type_filter"].as_str(), Some("autonomo"));

    let deadlines = parsed["deadlines"].as_array().expect("deadlines array");
    assert!(!deadlines.is_empty(), "autonomo should have deadlines");

    // Every returned deadline should be applicable to 'autonomo' (or 'all')
    for deadline in deadlines {
        let types = deadline["applicable_business_types"]
            .as_array()
            .expect("applicable_business_types array");
        let applicable = types.iter().any(|t| {
            let s = t.as_str().unwrap_or("");
            s == "autonomo" || s == "all"
        });
        assert!(
            applicable,
            "deadline model {} should be applicable to autonomo",
            deadline["model"],
        );
    }

    // Autonomo should include model 130 (pago fraccionado estimación directa)
    let has_130 = deadlines.iter().any(|d| d["model"].as_str() == Some("130"));
    assert!(has_130, "autonomo filter should include model 130");
}

// ---------------------------------------------------------------------------
// Verifactu tests (pure business logic, no HTTP)
// ---------------------------------------------------------------------------

#[test]
fn test_verifactu_sl_not_sii() {
    let result = tools::verifactu::execute("sl", None);
    let json = result.expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert_eq!(parsed["applicable"].as_bool(), Some(true));
    assert_eq!(parsed["sii_enrolled"].as_bool(), Some(false));

    let deadline = parsed["deadline"]
        .as_str()
        .expect("deadline should be present");
    assert!(
        deadline.contains("2027"),
        "SL deadline should be in 2027, got: {deadline}",
    );
}

#[test]
fn test_verifactu_sii_enrolled_exempt() {
    let result = tools::verifactu::execute("sl", Some(true));
    let json = result.expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert_eq!(
        parsed["applicable"].as_bool(),
        Some(false),
        "SII-enrolled businesses should be exempt from Verifactu",
    );
    assert_eq!(parsed["sii_enrolled"].as_bool(), Some(true));
    assert!(
        parsed["deadline"].is_null(),
        "exempt businesses should have no deadline",
    );
}

// ---------------------------------------------------------------------------
// BOE test (HTTP — uses wiremock)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_boe_search_with_mock() {
    let mock_server = MockServer::start().await;

    // Sample BOE API response matching the expected structure:
    // data.sumario.diario[].seccion[].departamento[].epigrafe[].item[]
    let sample_response = serde_json::json!({
        "data": {
            "sumario": {
                "diario": [
                    {
                        "seccion": [
                            {
                                "@nombre": "I. Disposiciones generales",
                                "departamento": {
                                    "@nombre": "Ministerio de Hacienda",
                                    "epigrafe": {
                                        "item": [
                                            {
                                                "@id": "BOE-A-2026-1234",
                                                "titulo": "Resolución sobre impuestos especiales y fiscalidad",
                                                "urlPdf": "/boe/dias/2026/03/23/pdfs/BOE-A-2026-1234.pdf"
                                            },
                                            {
                                                "@id": "BOE-A-2026-1235",
                                                "titulo": "Orden sobre regulación de mercados financieros",
                                                "urlPdf": "/boe/dias/2026/03/23/pdfs/BOE-A-2026-1235.pdf"
                                            }
                                        ]
                                    }
                                }
                            }
                        ]
                    }
                ]
            }
        }
    });

    Mock::given(method("GET"))
        .and(path_regex(r"/boe/sumario/\d{8}"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&sample_response))
        .mount(&mock_server)
        .await;

    let http = reqwest::Client::new();
    let cache = ResponseCache::new(100, Duration::from_mins(1));
    let config = BoeConfig {
        base_url: mock_server.uri(),
        rate_limit_rps: 10.0,
    };

    let result = tools::boe::execute(
        "impuestos",
        Some("20260323"),
        None,
        None,
        None,
        &http,
        &cache,
        &config,
    )
    .await;

    let json = result.expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let count = parsed["results_count"].as_u64().unwrap_or(0);
    assert!(count > 0, "expected at least one BOE document match");

    let documents = parsed["documents"].as_array().expect("documents array");
    let first_title = documents[0]["title"]
        .as_str()
        .expect("title should be a string");
    assert!(
        first_title.to_lowercase().contains("impuestos"),
        "first document should match keyword 'impuestos', got: {first_title}",
    );
    assert!(
        !documents[0]["identifier"].as_str().unwrap_or("").is_empty(),
        "document should have an identifier",
    );
    assert!(
        !documents[0]["pdf_url"].as_str().unwrap_or("").is_empty(),
        "document should have a PDF URL",
    );
}
