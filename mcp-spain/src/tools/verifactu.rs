//! `check_verifactu` tool — Verifactu e-invoicing compliance requirements.
//!
//! Pure business-logic tool (no external API call). Determines whether a
//! business must comply with the Verifactu regulation and by when, based on
//! business type and SII enrollment status.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct VerifactuResult {
    pub applicable: bool,
    pub deadline: Option<&'static str>,
    pub business_type: String,
    pub sii_enrolled: bool,
    pub sii_status: &'static str,
    pub requirements: Vec<&'static str>,
    pub documentation_urls: Vec<&'static str>,
}

/// Check Verifactu compliance requirements for a given business type.
///
/// # Errors
///
/// Returns an error if the response fails to serialize to JSON.
pub fn execute(business_type: &str, sii_enrolled: Option<bool>) -> Result<String, String> {
    let bt = business_type.to_lowercase();
    let sii = sii_enrolled.unwrap_or(false);

    // SII-enrolled businesses (facturación > 6M EUR) are exempt from Verifactu
    if sii {
        let result = VerifactuResult {
            applicable: false,
            deadline: None,
            business_type: bt,
            sii_enrolled: true,
            sii_status: "Enrolled in SII — exempt from Verifactu. SII already provides real-time invoice reporting.",
            requirements: vec![],
            documentation_urls: vec![
                "https://sede.agenciatributaria.gob.es/Sede/iva/suministro-inmediato-informacion.html",
            ],
        };
        return serde_json::to_string_pretty(&result).map_err(|e| format!("JSON error: {e}"));
    }

    // Determine deadline based on business type.
    // Corporate Tax (Impuesto de Sociedades) filers: 2027-07-01
    // All others (IRPF filers — autónomos): 2028-01-01
    let (deadline, deadline_note) = match bt.as_str() {
        "sl" | "sa" | "cooperativa" | "sociedad" => (
            "2027-07-01",
            "Corporate Tax filers (Impuesto de Sociedades) — earlier deadline.",
        ),
        "autonomo" | "autónomo" | "persona_fisica" => (
            "2028-01-01",
            "IRPF filers (autónomos / personas físicas) — later deadline.",
        ),
        _ => (
            "2028-01-01",
            "Unknown business type — assuming IRPF filer deadline. Verify with your assessor.",
        ),
    };

    let requirements = vec![
        "Use Verifactu-compliant invoicing software (certified by AEAT)",
        "Each invoice must generate a chained hash (huella) for tamper detection",
        "Invoicing system must be able to send records to AEAT in real time or upon request",
        "Maintain complete invoice registry (Libro Registro) in electronic format",
        "Software must display 'Veri*factu' identifier on compliant invoices",
        "Conservative record: invoices must be stored for 4 years minimum",
        deadline_note,
    ];

    let result = VerifactuResult {
        applicable: true,
        deadline: Some(deadline),
        business_type: bt,
        sii_enrolled: false,
        sii_status: "Not enrolled in SII — Verifactu compliance is mandatory.",
        requirements,
        documentation_urls: vec![
            "https://sede.agenciatributaria.gob.es/Sede/iva/facturacion-registro/verifactu.html",
            "https://www.boe.es/buscar/act.php?id=BOE-A-2024-22138",
            "https://www.agenciatributaria.es/AEAT.internet/Inicio/La_Agencia_Tributaria/Campanas/Verifactu.shtml",
        ],
    };

    serde_json::to_string_pretty(&result).map_err(|e| format!("JSON error: {e}"))
}
