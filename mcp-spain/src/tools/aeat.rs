//! `get_aeat_calendar` tool — Spanish tax calendar / deadlines.
//!
//! Embeds a structured dataset of the main Spanish tax model deadlines,
//! since the AEAT website is scraping-hostile. Supports filtering by year,
//! quarter, and business type.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TaxDeadline {
    pub model: &'static str,
    pub description: &'static str,
    pub deadline_month: u8,
    pub deadline_day: u8,
    pub quarter: u8,
    pub applicable_business_types: &'static [&'static str],
    pub frequency: &'static str,
    pub notes: &'static str,
}

/// Get AEAT tax calendar deadlines, optionally filtered by quarter and business type.
///
/// # Errors
///
/// Returns an error if `year` is outside the supported 2024-2030 range, if
/// `quarter` is not in 1-4, or if the response fails to serialize to JSON.
pub fn execute(
    year: u16,
    quarter: Option<u8>,
    business_type: Option<&str>,
) -> Result<String, String> {
    if !(2024..=2030).contains(&year) {
        return Err(format!(
            "Year {year} is out of supported range (2024-2030). Tax deadlines may vary."
        ));
    }

    if let Some(q) = quarter
        && !(1..=4).contains(&q)
    {
        return Err(format!("Quarter must be 1-4, got {q}."));
    }

    let bt_lower = business_type.map(str::to_lowercase);

    let results: Vec<_> = TAX_DEADLINES
        .iter()
        .filter(|d| quarter.is_none_or(|q| d.quarter == q))
        .filter(|d| {
            bt_lower.as_ref().is_none_or(|bt| {
                d.applicable_business_types
                    .iter()
                    .any(|t| t.to_lowercase() == *bt || *t == "all")
            })
        })
        .map(|d| {
            serde_json::json!({
                "model": d.model,
                "description": d.description,
                "deadline": format!("{year}-{:02}-{:02}", d.deadline_month, d.deadline_day),
                "quarter": d.quarter,
                "applicable_business_types": d.applicable_business_types,
                "frequency": d.frequency,
                "notes": d.notes,
            })
        })
        .collect();

    serde_json::to_string_pretty(&serde_json::json!({
        "year": year,
        "quarter_filter": quarter,
        "business_type_filter": business_type,
        "deadlines_count": results.len(),
        "deadlines": results,
    }))
    .map_err(|e| format!("JSON error: {e}"))
}

// Main Spanish tax model deadlines.
// Deadlines are for the quarter following the fiscal period (e.g., Q1 declared in April).
// Annual models are included in Q4 with their specific dates.
static TAX_DEADLINES: &[TaxDeadline] = &[
    // --- Q1 declarations (for Q4 of previous year / January-March) - Due April ---
    TaxDeadline {
        model: "303",
        description: "IVA — Autoliquidación trimestral",
        deadline_month: 4,
        deadline_day: 20,
        quarter: 1,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "Pago fraccionado de IVA. Q4 se presenta hasta el 30 de enero.",
    },
    TaxDeadline {
        model: "111",
        description: "IRPF — Retenciones e ingresos a cuenta (rendimientos del trabajo, actividades profesionales)",
        deadline_month: 4,
        deadline_day: 20,
        quarter: 1,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "Obligatorio si se practican retenciones a empleados o profesionales.",
    },
    TaxDeadline {
        model: "115",
        description: "IRPF — Retenciones e ingresos a cuenta (rentas o rendimientos de arrendamientos)",
        deadline_month: 4,
        deadline_day: 20,
        quarter: 1,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "Solo si se paga alquiler de local/oficina con retención.",
    },
    TaxDeadline {
        model: "130",
        description: "IRPF — Pago fraccionado (estimación directa)",
        deadline_month: 4,
        deadline_day: 20,
        quarter: 1,
        applicable_business_types: &["autonomo"],
        frequency: "quarterly",
        notes: "Autónomos en estimación directa. 20% del rendimiento neto.",
    },
    TaxDeadline {
        model: "131",
        description: "IRPF — Pago fraccionado (estimación objetiva / módulos)",
        deadline_month: 4,
        deadline_day: 20,
        quarter: 1,
        applicable_business_types: &["autonomo"],
        frequency: "quarterly",
        notes: "Autónomos en estimación objetiva (módulos).",
    },
    // --- Q2 declarations (for April-June) - Due July ---
    TaxDeadline {
        model: "303",
        description: "IVA — Autoliquidación trimestral",
        deadline_month: 7,
        deadline_day: 20,
        quarter: 2,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "Pago fraccionado de IVA.",
    },
    TaxDeadline {
        model: "111",
        description: "IRPF — Retenciones e ingresos a cuenta (rendimientos del trabajo, actividades profesionales)",
        deadline_month: 7,
        deadline_day: 20,
        quarter: 2,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "",
    },
    TaxDeadline {
        model: "115",
        description: "IRPF — Retenciones e ingresos a cuenta (arrendamientos)",
        deadline_month: 7,
        deadline_day: 20,
        quarter: 2,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "",
    },
    TaxDeadline {
        model: "130",
        description: "IRPF — Pago fraccionado (estimación directa)",
        deadline_month: 7,
        deadline_day: 20,
        quarter: 2,
        applicable_business_types: &["autonomo"],
        frequency: "quarterly",
        notes: "",
    },
    TaxDeadline {
        model: "131",
        description: "IRPF — Pago fraccionado (estimación objetiva / módulos)",
        deadline_month: 7,
        deadline_day: 20,
        quarter: 2,
        applicable_business_types: &["autonomo"],
        frequency: "quarterly",
        notes: "",
    },
    TaxDeadline {
        model: "200",
        description: "Impuesto sobre Sociedades — Declaración anual",
        deadline_month: 7,
        deadline_day: 25,
        quarter: 2,
        applicable_business_types: &["sl", "sa", "cooperativa"],
        frequency: "annual",
        notes: "Plazo general: 25 días naturales siguientes a los 6 meses desde cierre del ejercicio (julio para ejercicios coincidentes con año natural).",
    },
    // --- Q3 declarations (for July-September) - Due October ---
    TaxDeadline {
        model: "303",
        description: "IVA — Autoliquidación trimestral",
        deadline_month: 10,
        deadline_day: 20,
        quarter: 3,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "Pago fraccionado de IVA.",
    },
    TaxDeadline {
        model: "111",
        description: "IRPF — Retenciones e ingresos a cuenta (rendimientos del trabajo, actividades profesionales)",
        deadline_month: 10,
        deadline_day: 20,
        quarter: 3,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "",
    },
    TaxDeadline {
        model: "115",
        description: "IRPF — Retenciones e ingresos a cuenta (arrendamientos)",
        deadline_month: 10,
        deadline_day: 20,
        quarter: 3,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "",
    },
    TaxDeadline {
        model: "130",
        description: "IRPF — Pago fraccionado (estimación directa)",
        deadline_month: 10,
        deadline_day: 20,
        quarter: 3,
        applicable_business_types: &["autonomo"],
        frequency: "quarterly",
        notes: "",
    },
    TaxDeadline {
        model: "131",
        description: "IRPF — Pago fraccionado (estimación objetiva / módulos)",
        deadline_month: 10,
        deadline_day: 20,
        quarter: 3,
        applicable_business_types: &["autonomo"],
        frequency: "quarterly",
        notes: "",
    },
    TaxDeadline {
        model: "202",
        description: "Impuesto sobre Sociedades — Pago fraccionado",
        deadline_month: 10,
        deadline_day: 20,
        quarter: 3,
        applicable_business_types: &["sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "Primer pago fraccionado del IS. Se presenta también en abril (Q1) y diciembre (Q4).",
    },
    // --- Q4 declarations (for October-December) - Due January / annual summaries ---
    TaxDeadline {
        model: "303",
        description: "IVA — Autoliquidación trimestral (Q4)",
        deadline_month: 1,
        deadline_day: 30,
        quarter: 4,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "Plazo especial Q4: hasta el 30 de enero.",
    },
    TaxDeadline {
        model: "390",
        description: "IVA — Declaración resumen anual",
        deadline_month: 1,
        deadline_day: 30,
        quarter: 4,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "annual",
        notes: "Resumen anual de IVA. Se presenta junto con el 303 de Q4.",
    },
    TaxDeadline {
        model: "111",
        description: "IRPF — Retenciones e ingresos a cuenta (Q4)",
        deadline_month: 1,
        deadline_day: 20,
        quarter: 4,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "",
    },
    TaxDeadline {
        model: "190",
        description: "IRPF — Resumen anual de retenciones e ingresos a cuenta",
        deadline_month: 1,
        deadline_day: 31,
        quarter: 4,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "annual",
        notes: "Declaración informativa anual que resume los modelos 111.",
    },
    TaxDeadline {
        model: "115",
        description: "IRPF — Retenciones arrendamientos (Q4)",
        deadline_month: 1,
        deadline_day: 20,
        quarter: 4,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "",
    },
    TaxDeadline {
        model: "180",
        description: "IRPF — Resumen anual de retenciones por arrendamientos",
        deadline_month: 1,
        deadline_day: 31,
        quarter: 4,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "annual",
        notes: "Declaración informativa anual que resume los modelos 115.",
    },
    TaxDeadline {
        model: "130",
        description: "IRPF — Pago fraccionado estimación directa (Q4)",
        deadline_month: 1,
        deadline_day: 30,
        quarter: 4,
        applicable_business_types: &["autonomo"],
        frequency: "quarterly",
        notes: "Plazo especial Q4: hasta el 30 de enero.",
    },
    TaxDeadline {
        model: "131",
        description: "IRPF — Pago fraccionado estimación objetiva (Q4)",
        deadline_month: 1,
        deadline_day: 30,
        quarter: 4,
        applicable_business_types: &["autonomo"],
        frequency: "quarterly",
        notes: "Plazo especial Q4: hasta el 30 de enero.",
    },
    TaxDeadline {
        model: "349",
        description: "Declaración recapitulativa de operaciones intracomunitarias",
        deadline_month: 1,
        deadline_day: 30,
        quarter: 4,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "quarterly",
        notes: "Solo obligatorio si se realizan operaciones intracomunitarias. Se presenta trimestralmente o mensualmente según volumen.",
    },
    TaxDeadline {
        model: "347",
        description: "Declaración anual de operaciones con terceras personas",
        deadline_month: 2,
        deadline_day: 28,
        quarter: 4,
        applicable_business_types: &["autonomo", "sl", "sa", "cooperativa"],
        frequency: "annual",
        notes: "Obligatorio si operaciones con un mismo tercero superan 3.005,06 EUR anuales. Se presenta en febrero.",
    },
];
