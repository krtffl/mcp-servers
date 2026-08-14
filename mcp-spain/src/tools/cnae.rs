//! `lookup_cnae` tool — search CNAE 2009 business classification codes.
//!
//! Uses an embedded static dataset of the most common CNAE codes rather than
//! querying the INE API (which is unreliable). Supports search by code prefix
//! or case-insensitive description substring.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CnaeEntry {
    pub code: &'static str,
    pub description: &'static str,
    pub section: &'static str,
    pub division: &'static str,
    pub group: &'static str,
}

/// Search CNAE codes by code prefix, description substring, or both.
///
/// # Errors
///
/// Returns an error if neither `code` nor `description` is provided, or if the
/// response fails to serialize to JSON.
pub fn execute(code: Option<&str>, description: Option<&str>) -> Result<String, String> {
    if code.is_none() && description.is_none() {
        return Err("At least one of 'code' or 'description' must be provided.".to_string());
    }

    let query_lower = description.map(str::to_lowercase);

    let results: Vec<&CnaeEntry> = CNAE_CODES
        .iter()
        .filter(|entry| {
            let code_match = code.as_ref().is_none_or(|c| entry.code.starts_with(&**c));
            let desc_match = query_lower
                .as_ref()
                .is_none_or(|q| entry.description.to_lowercase().contains(q.as_str()));
            code_match && desc_match
        })
        .collect();

    if results.is_empty() {
        return Ok(serde_json::json!({
            "matches": 0,
            "results": []
        })
        .to_string());
    }

    serde_json::to_string_pretty(&serde_json::json!({
        "matches": results.len(),
        "results": results,
    }))
    .map_err(|e| format!("JSON error: {e}"))
}

// CNAE 2009 — embedded static dataset of the most common business activity codes.
// Source: INE (Instituto Nacional de Estadística) CNAE-2009.
static CNAE_CODES: &[CnaeEntry] = &[
    // A — Agricultura, ganadería, silvicultura y pesca
    CnaeEntry {
        code: "0111",
        description: "Cultivo de cereales (excepto arroz), leguminosas y semillas oleaginosas",
        section: "A",
        division: "01",
        group: "011",
    },
    CnaeEntry {
        code: "0113",
        description: "Cultivo de hortalizas, raíces y tubérculos",
        section: "A",
        division: "01",
        group: "011",
    },
    CnaeEntry {
        code: "0121",
        description: "Cultivo de la vid",
        section: "A",
        division: "01",
        group: "012",
    },
    CnaeEntry {
        code: "0141",
        description: "Explotación de ganado bovino para la producción de leche",
        section: "A",
        division: "01",
        group: "014",
    },
    // C — Industria manufacturera
    CnaeEntry {
        code: "1011",
        description: "Procesado y conservación de carne",
        section: "C",
        division: "10",
        group: "101",
    },
    CnaeEntry {
        code: "1039",
        description: "Otro procesado y conservación de frutas y hortalizas",
        section: "C",
        division: "10",
        group: "103",
    },
    CnaeEntry {
        code: "1071",
        description: "Fabricación de pan y de productos frescos de panadería y pastelería",
        section: "C",
        division: "10",
        group: "107",
    },
    CnaeEntry {
        code: "1101",
        description: "Destilación, rectificación y mezcla de bebidas alcohólicas",
        section: "C",
        division: "11",
        group: "110",
    },
    CnaeEntry {
        code: "1102",
        description: "Elaboración de vinos",
        section: "C",
        division: "11",
        group: "110",
    },
    CnaeEntry {
        code: "2511",
        description: "Fabricación de estructuras metálicas y sus componentes",
        section: "C",
        division: "25",
        group: "251",
    },
    CnaeEntry {
        code: "2562",
        description: "Ingeniería mecánica por cuenta de terceros",
        section: "C",
        division: "25",
        group: "256",
    },
    // F — Construcción
    CnaeEntry {
        code: "4110",
        description: "Promoción inmobiliaria",
        section: "F",
        division: "41",
        group: "411",
    },
    CnaeEntry {
        code: "4121",
        description: "Construcción de edificios residenciales",
        section: "F",
        division: "41",
        group: "412",
    },
    CnaeEntry {
        code: "4122",
        description: "Construcción de edificios no residenciales",
        section: "F",
        division: "41",
        group: "412",
    },
    CnaeEntry {
        code: "4221",
        description: "Construcción de redes eléctricas y de telecomunicaciones",
        section: "F",
        division: "42",
        group: "422",
    },
    CnaeEntry {
        code: "4321",
        description: "Instalaciones eléctricas",
        section: "F",
        division: "43",
        group: "432",
    },
    CnaeEntry {
        code: "4322",
        description: "Fontanería, instalaciones de sistemas de calefacción y aire acondicionado",
        section: "F",
        division: "43",
        group: "432",
    },
    CnaeEntry {
        code: "4332",
        description: "Instalación de carpintería",
        section: "F",
        division: "43",
        group: "433",
    },
    CnaeEntry {
        code: "4399",
        description: "Otras actividades de construcción especializada n.c.o.p.",
        section: "F",
        division: "43",
        group: "439",
    },
    // G — Comercio al por mayor y al por menor
    CnaeEntry {
        code: "4511",
        description: "Venta de automóviles y vehículos de motor ligeros",
        section: "G",
        division: "45",
        group: "451",
    },
    CnaeEntry {
        code: "4619",
        description: "Intermediarios del comercio de productos diversos",
        section: "G",
        division: "46",
        group: "461",
    },
    CnaeEntry {
        code: "4631",
        description: "Comercio al por mayor de frutas y hortalizas",
        section: "G",
        division: "46",
        group: "463",
    },
    CnaeEntry {
        code: "4690",
        description: "Comercio al por mayor no especializado",
        section: "G",
        division: "46",
        group: "469",
    },
    CnaeEntry {
        code: "4711",
        description: "Comercio al por menor en establecimientos no especializados, con predominio en productos alimenticios",
        section: "G",
        division: "47",
        group: "471",
    },
    CnaeEntry {
        code: "4719",
        description: "Otro comercio al por menor en establecimientos no especializados",
        section: "G",
        division: "47",
        group: "471",
    },
    CnaeEntry {
        code: "4771",
        description: "Comercio al por menor de prendas de vestir en establecimientos especializados",
        section: "G",
        division: "47",
        group: "477",
    },
    CnaeEntry {
        code: "4791",
        description: "Comercio al por menor por correspondencia o por Internet",
        section: "G",
        division: "47",
        group: "479",
    },
    // H — Transporte y almacenamiento
    CnaeEntry {
        code: "4941",
        description: "Transporte de mercancías por carretera",
        section: "H",
        division: "49",
        group: "494",
    },
    CnaeEntry {
        code: "5210",
        description: "Depósito y almacenamiento",
        section: "H",
        division: "52",
        group: "521",
    },
    CnaeEntry {
        code: "5320",
        description: "Otras actividades postales y de correos",
        section: "H",
        division: "53",
        group: "532",
    },
    // I — Hostelería
    CnaeEntry {
        code: "5510",
        description: "Hoteles y alojamientos similares",
        section: "I",
        division: "55",
        group: "551",
    },
    CnaeEntry {
        code: "5520",
        description: "Alojamientos turísticos y otros alojamientos de corta estancia",
        section: "I",
        division: "55",
        group: "552",
    },
    CnaeEntry {
        code: "5610",
        description: "Restaurantes y puestos de comidas",
        section: "I",
        division: "56",
        group: "561",
    },
    CnaeEntry {
        code: "5630",
        description: "Establecimientos de bebidas",
        section: "I",
        division: "56",
        group: "563",
    },
    // J — Información y comunicaciones
    CnaeEntry {
        code: "6201",
        description: "Actividades de programación informática",
        section: "J",
        division: "62",
        group: "620",
    },
    CnaeEntry {
        code: "6202",
        description: "Actividades de consultoría informática",
        section: "J",
        division: "62",
        group: "620",
    },
    CnaeEntry {
        code: "6209",
        description: "Otros servicios relacionados con las tecnologías de la información y la informática",
        section: "J",
        division: "62",
        group: "620",
    },
    CnaeEntry {
        code: "6311",
        description: "Proceso de datos, hosting y actividades relacionadas",
        section: "J",
        division: "63",
        group: "631",
    },
    CnaeEntry {
        code: "6312",
        description: "Portales web",
        section: "J",
        division: "63",
        group: "631",
    },
    // K — Actividades financieras y de seguros
    CnaeEntry {
        code: "6419",
        description: "Otra intermediación monetaria",
        section: "K",
        division: "64",
        group: "641",
    },
    CnaeEntry {
        code: "6499",
        description: "Otros servicios financieros, excepto seguros y fondos de pensiones n.c.o.p.",
        section: "K",
        division: "64",
        group: "649",
    },
    CnaeEntry {
        code: "6622",
        description: "Actividades de agentes y corredores de seguros",
        section: "K",
        division: "66",
        group: "662",
    },
    // L — Actividades inmobiliarias
    CnaeEntry {
        code: "6810",
        description: "Compraventa de bienes inmobiliarios por cuenta propia",
        section: "L",
        division: "68",
        group: "681",
    },
    CnaeEntry {
        code: "6820",
        description: "Alquiler de bienes inmobiliarios por cuenta propia",
        section: "L",
        division: "68",
        group: "682",
    },
    CnaeEntry {
        code: "6831",
        description: "Agentes de la propiedad inmobiliaria",
        section: "L",
        division: "68",
        group: "683",
    },
    CnaeEntry {
        code: "6832",
        description: "Gestión y administración de la propiedad inmobiliaria",
        section: "L",
        division: "68",
        group: "683",
    },
    // M — Actividades profesionales, científicas y técnicas
    CnaeEntry {
        code: "6910",
        description: "Actividades jurídicas",
        section: "M",
        division: "69",
        group: "691",
    },
    CnaeEntry {
        code: "6920",
        description: "Actividades de contabilidad, teneduría de libros, auditoría y asesoría fiscal",
        section: "M",
        division: "69",
        group: "692",
    },
    CnaeEntry {
        code: "7010",
        description: "Actividades de las sedes centrales",
        section: "M",
        division: "70",
        group: "701",
    },
    CnaeEntry {
        code: "7021",
        description: "Relaciones públicas y comunicación",
        section: "M",
        division: "70",
        group: "702",
    },
    CnaeEntry {
        code: "7022",
        description: "Otras actividades de consultoría de gestión empresarial",
        section: "M",
        division: "70",
        group: "702",
    },
    CnaeEntry {
        code: "7111",
        description: "Servicios técnicos de arquitectura",
        section: "M",
        division: "71",
        group: "711",
    },
    CnaeEntry {
        code: "7112",
        description: "Servicios técnicos de ingeniería y otras actividades relacionadas con el asesoramiento técnico",
        section: "M",
        division: "71",
        group: "711",
    },
    CnaeEntry {
        code: "7311",
        description: "Agencias de publicidad",
        section: "M",
        division: "73",
        group: "731",
    },
    CnaeEntry {
        code: "7490",
        description: "Otras actividades profesionales, científicas y técnicas n.c.o.p.",
        section: "M",
        division: "74",
        group: "749",
    },
    // N — Actividades administrativas y servicios auxiliares
    CnaeEntry {
        code: "7810",
        description: "Actividades de agencias de colocación",
        section: "N",
        division: "78",
        group: "781",
    },
    CnaeEntry {
        code: "8010",
        description: "Actividades de seguridad privada",
        section: "N",
        division: "80",
        group: "801",
    },
    CnaeEntry {
        code: "8121",
        description: "Limpieza general de edificios",
        section: "N",
        division: "81",
        group: "812",
    },
    CnaeEntry {
        code: "8211",
        description: "Servicios administrativos combinados",
        section: "N",
        division: "82",
        group: "821",
    },
    // P — Educación
    CnaeEntry {
        code: "8551",
        description: "Educación deportiva y recreativa",
        section: "P",
        division: "85",
        group: "855",
    },
    CnaeEntry {
        code: "8559",
        description: "Otra educación n.c.o.p.",
        section: "P",
        division: "85",
        group: "855",
    },
    // Q — Actividades sanitarias y de servicios sociales
    CnaeEntry {
        code: "8621",
        description: "Actividades de medicina general",
        section: "Q",
        division: "86",
        group: "862",
    },
    CnaeEntry {
        code: "8622",
        description: "Actividades de medicina especializada",
        section: "Q",
        division: "86",
        group: "862",
    },
    CnaeEntry {
        code: "8623",
        description: "Actividades odontológicas",
        section: "Q",
        division: "86",
        group: "862",
    },
    // R — Actividades artísticas, recreativas y de entretenimiento
    CnaeEntry {
        code: "9001",
        description: "Artes escénicas",
        section: "R",
        division: "90",
        group: "900",
    },
    CnaeEntry {
        code: "9313",
        description: "Actividades de los gimnasios",
        section: "R",
        division: "93",
        group: "931",
    },
    // S — Otros servicios
    CnaeEntry {
        code: "9511",
        description: "Reparación de ordenadores y equipos periféricos",
        section: "S",
        division: "95",
        group: "951",
    },
    CnaeEntry {
        code: "9602",
        description: "Peluquería y otros tratamientos de belleza",
        section: "S",
        division: "96",
        group: "960",
    },
];
