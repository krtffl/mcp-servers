//! mcp-spain: MCP server for Spanish government data.
//!
//! Tools: `lookup_cnae`, `search_boe`, `get_aeat_calendar`, `query_catastro`, `check_verifactu`.

mod config;
mod tools;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use mcp_common::{ResponseCache, build_http_client};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{ServerHandler, ServiceExt as _, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use config::SpainConfig;

#[derive(Parser, Debug)]
#[command(name = "mcp-spain", version, about = "MCP server for Spanish government data")]
struct Cli {
    /// Path to TOML config file.
    #[arg(short, long)]
    config: Option<std::path::PathBuf>,

    /// Log level (trace, debug, info, warn, error).
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

// --- Tool input types ---

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LookupCnaeInput {
    /// CNAE code or prefix to search for (e.g., "6201", "62").
    pub code: Option<String>,
    /// Description substring to search for (case-insensitive).
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchBoeInput {
    /// Keywords to search in BOE document titles.
    pub keywords: String,
    /// Start date for search range (YYYYMMDD or YYYY-MM-DD).
    pub date_from: Option<String>,
    /// End date for search range (YYYYMMDD or YYYY-MM-DD).
    pub date_to: Option<String>,
    /// Filter by BOE section name.
    pub section: Option<String>,
    /// Filter by publishing department.
    pub department: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetAeatCalendarInput {
    /// Tax year (e.g., 2025).
    pub year: u16,
    /// Quarter to filter (1-4). If omitted, returns all quarters.
    pub quarter: Option<u8>,
    /// Business type filter: autonomo, sl, sa, cooperativa.
    pub business_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryCatastroInput {
    /// Cadastral reference number (20-character alphanumeric).
    pub reference: Option<String>,
    /// Province name for address-based lookup.
    pub province: Option<String>,
    /// Municipality name for address-based lookup.
    pub municipality: Option<String>,
    /// Street name for address-based lookup.
    pub street: Option<String>,
    /// Street number for address-based lookup.
    pub number: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckVerifactuInput {
    /// Business type: autonomo, sl, sa, cooperativa.
    pub business_type: String,
    /// Whether the business is enrolled in SII (facturación > 6M EUR).
    pub sii_enrolled: Option<bool>,
}

// --- Server ---

#[derive(Clone)]
pub struct SpainServer {
    tool_router: ToolRouter<Self>,
    config: Arc<SpainConfig>,
    http: reqwest::Client,
    cache: Arc<ResponseCache>,
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for SpainServer {}

#[tool_router(router = tool_router)]
impl SpainServer {
    #[must_use]
    pub fn new(config: SpainConfig, http: reqwest::Client, cache: ResponseCache) -> Self {
        Self {
            tool_router: Self::tool_router(),
            config: Arc::new(config),
            http,
            cache: Arc::new(cache),
        }
    }

    #[tool(
        name = "lookup_cnae",
        description = "Look up CNAE 2009 business classification codes by code prefix or description keyword"
    )]
    async fn lookup_cnae(
        &self,
        input: Parameters<LookupCnaeInput>,
    ) -> Result<String, String> {
        tools::cnae::execute(input.0.code.as_deref(), input.0.description.as_deref())
    }

    #[tool(
        name = "search_boe",
        description = "Search the BOE (Boletín Oficial del Estado) for Spanish laws, regulations, and official publications"
    )]
    async fn search_boe(
        &self,
        input: Parameters<SearchBoeInput>,
    ) -> Result<String, String> {
        tools::boe::execute(
            &input.0.keywords,
            input.0.date_from.as_deref(),
            input.0.date_to.as_deref(),
            input.0.section.as_deref(),
            input.0.department.as_deref(),
            &self.http,
            &self.cache,
            &self.config.boe,
        )
        .await
    }

    #[tool(
        name = "get_aeat_calendar",
        description = "Get Spanish tax calendar deadlines from AEAT (models 303, 111, 115, 200, 130, 131, etc.) filtered by year, quarter, and business type"
    )]
    async fn get_aeat_calendar(
        &self,
        input: Parameters<GetAeatCalendarInput>,
    ) -> Result<String, String> {
        tools::aeat::execute(
            input.0.year,
            input.0.quarter,
            input.0.business_type.as_deref(),
        )
    }

    #[tool(
        name = "query_catastro",
        description = "Query the Spanish Catastro (property registry) by cadastral reference or street address"
    )]
    async fn query_catastro(
        &self,
        input: Parameters<QueryCatastroInput>,
    ) -> Result<String, String> {
        tools::catastro::execute(
            input.0.reference.as_deref(),
            input.0.province.as_deref(),
            input.0.municipality.as_deref(),
            input.0.street.as_deref(),
            input.0.number.as_deref(),
            &self.http,
            &self.cache,
            &self.config.catastro,
        )
        .await
    }

    #[tool(
        name = "check_verifactu",
        description = "Check Verifactu e-invoicing compliance requirements and deadlines for a Spanish business"
    )]
    async fn check_verifactu(
        &self,
        input: Parameters<CheckVerifactuInput>,
    ) -> Result<String, String> {
        tools::verifactu::execute(&input.0.business_type, input.0.sii_enrolled)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let config: SpainConfig = if let Some(path) = &cli.config {
        mcp_common::config::load_config(path)?
    } else {
        tracing::info!("no config file specified, using defaults");
        toml::from_str(DEFAULT_CONFIG)?
    };

    let http = build_http_client()?;
    let cache = ResponseCache::new(
        config.server.cache.max_entries,
        Duration::from_secs(config.server.cache.ttl_seconds),
    );

    let server = SpainServer::new(config, http, cache);

    tracing::info!("starting mcp-spain server (stdio transport)");
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;

    Ok(())
}

const DEFAULT_CONFIG: &str = r#"
transport = "stdio"

[cache]
max_entries = 5000
ttl_seconds = 3600

[boe]
base_url = "https://boe.es/datosabiertos/api"
rate_limit_rps = 2.0

[aeat]
base_url = "https://sede.agenciatributaria.gob.es"
rate_limit_rps = 1.0

[cnae]
base_url = "https://www.ine.es"
rate_limit_rps = 2.0

[catastro]
base_url = "https://ovc.catastro.meh.es/ovcservweb"
rate_limit_rps = 1.0

[verifactu]
base_url = "https://sede.agenciatributaria.gob.es"
rate_limit_rps = 1.0
"#;
