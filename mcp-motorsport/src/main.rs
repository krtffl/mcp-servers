//! mcp-motorsport: MCP server for F1 motorsport data.
//!
//! Tools: `get_race_results`, `get_session_results`, `get_lap_times`,
//! `get_tire_stints`, `get_telemetry`, `compare_drivers`, `get_standings`.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use mcp_common::{ResponseCache, build_http_client};
use mcp_motorsport::config;
use mcp_motorsport::tools;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{ServerHandler, ServiceExt as _, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use config::MotorsportConfig;

#[derive(Parser, Debug)]
#[command(name = "mcp-motorsport", version, about = "MCP server for F1 motorsport data")]
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
pub struct GetRaceResultsInput {
    /// Season year (e.g., 2024).
    pub year: u16,
    /// Grand Prix country name (e.g., "Bahrain", "Monaco", "Spain").
    pub grand_prix: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetSessionResultsInput {
    /// Season year (e.g., 2024).
    pub year: u16,
    /// Grand Prix country name (e.g., "Bahrain", "Monaco", "Spain").
    pub grand_prix: String,
    /// Session type: FP1, FP2, FP3, Qualifying, Sprint, or Race.
    pub session: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLapTimesInput {
    /// Season year (e.g., 2024).
    pub year: u16,
    /// Grand Prix country name (e.g., "Bahrain", "Monaco", "Spain").
    pub grand_prix: String,
    /// Session type: FP1, FP2, FP3, Qualifying, Sprint, or Race.
    pub session: String,
    /// Driver name, three-letter acronym (e.g., "VER"), or car number (e.g., "1").
    pub driver: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTireStintsInput {
    /// Season year (e.g., 2024).
    pub year: u16,
    /// Grand Prix country name (e.g., "Bahrain", "Monaco", "Spain").
    pub grand_prix: String,
    /// Optional driver name, acronym, or number to filter results. If omitted, returns all drivers.
    pub driver: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTelemetryInput {
    /// Season year (e.g., 2024).
    pub year: u16,
    /// Grand Prix country name (e.g., "Bahrain", "Monaco", "Spain").
    pub grand_prix: String,
    /// Session type: FP1, FP2, FP3, Qualifying, Sprint, or Race.
    pub session: String,
    /// Driver name, three-letter acronym (e.g., "VER"), or car number (e.g., "1").
    pub driver: String,
    /// Optional lap number to filter telemetry to a single lap.
    pub lap: Option<u16>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompareDriversInput {
    /// Season year (e.g., 2024).
    pub year: u16,
    /// Grand Prix country name (e.g., "Bahrain", "Monaco", "Spain").
    pub grand_prix: String,
    /// Session type: FP1, FP2, FP3, Qualifying, Sprint, or Race.
    pub session: String,
    /// First driver: name, acronym, or car number.
    pub driver_a: String,
    /// Second driver: name, acronym, or car number.
    pub driver_b: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetStandingsInput {
    /// Season year (e.g., 2024).
    pub year: u16,
    /// Type of standings: "drivers" or "constructors".
    pub standings_type: String,
}

// --- Server ---

#[derive(Clone)]
pub struct MotorsportServer {
    tool_router: ToolRouter<Self>,
    config: Arc<MotorsportConfig>,
    http: reqwest::Client,
    cache: Arc<ResponseCache>,
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for MotorsportServer {}

#[tool_router(router = tool_router)]
impl MotorsportServer {
    #[must_use]
    pub fn new(config: MotorsportConfig, http: reqwest::Client, cache: ResponseCache) -> Self {
        Self {
            tool_router: Self::tool_router(),
            config: Arc::new(config),
            http,
            cache: Arc::new(cache),
        }
    }

    #[tool(
        name = "get_race_results",
        description = "Get the final race classification for a Formula 1 Grand Prix, including finishing positions, driver names, and teams"
    )]
    async fn get_race_results(
        &self,
        input: Parameters<GetRaceResultsInput>,
    ) -> Result<String, String> {
        tools::race_results::execute(
            input.0.year,
            &input.0.grand_prix,
            &self.http,
            &self.cache,
            &self.config.openf1,
        )
        .await
    }

    #[tool(
        name = "get_session_results",
        description = "Get results for any F1 session (FP1, FP2, FP3, Qualifying, Sprint, Race) with finishing positions and driver info"
    )]
    async fn get_session_results(
        &self,
        input: Parameters<GetSessionResultsInput>,
    ) -> Result<String, String> {
        tools::session_results::execute(
            input.0.year,
            &input.0.grand_prix,
            &input.0.session,
            &self.http,
            &self.cache,
            &self.config.openf1,
        )
        .await
    }

    #[tool(
        name = "get_lap_times",
        description = "Get detailed lap-by-lap timing data for a specific driver in an F1 session, including sector times"
    )]
    async fn get_lap_times(
        &self,
        input: Parameters<GetLapTimesInput>,
    ) -> Result<String, String> {
        tools::lap_times::execute(
            input.0.year,
            &input.0.grand_prix,
            &input.0.session,
            &input.0.driver,
            &self.http,
            &self.cache,
            &self.config.openf1,
        )
        .await
    }

    #[tool(
        name = "get_tire_stints",
        description = "Get tire strategy data for an F1 race: compound type, stint length, and lap ranges per driver"
    )]
    async fn get_tire_stints(
        &self,
        input: Parameters<GetTireStintsInput>,
    ) -> Result<String, String> {
        tools::tire_stints::execute(
            input.0.year,
            &input.0.grand_prix,
            input.0.driver.as_deref(),
            &self.http,
            &self.cache,
            &self.config.openf1,
        )
        .await
    }

    #[tool(
        name = "get_telemetry",
        description = "Get car telemetry data (speed, throttle, brake, gear, DRS) for a driver in an F1 session, optionally filtered to a specific lap"
    )]
    async fn get_telemetry(
        &self,
        input: Parameters<GetTelemetryInput>,
    ) -> Result<String, String> {
        tools::telemetry::execute(
            input.0.year,
            &input.0.grand_prix,
            &input.0.session,
            &input.0.driver,
            input.0.lap,
            &self.http,
            &self.cache,
            &self.config.openf1,
        )
        .await
    }

    #[tool(
        name = "compare_drivers",
        description = "Head-to-head comparison of two F1 drivers in a session: per-lap deltas, average pace, and best lap times"
    )]
    async fn compare_drivers(
        &self,
        input: Parameters<CompareDriversInput>,
    ) -> Result<String, String> {
        tools::compare_drivers::execute(
            input.0.year,
            &input.0.grand_prix,
            &input.0.session,
            &input.0.driver_a,
            &input.0.driver_b,
            &self.http,
            &self.cache,
            &self.config.openf1,
        )
        .await
    }

    #[tool(
        name = "get_standings",
        description = "Get current F1 championship standings (driver or constructor) for a season, including points and wins"
    )]
    async fn get_standings(
        &self,
        input: Parameters<GetStandingsInput>,
    ) -> Result<String, String> {
        let jolpica = self
            .config
            .jolpica
            .as_ref()
            .ok_or("Jolpica-F1 API is not configured. Add a [jolpica] section to your config file.")?;
        tools::standings::execute(
            input.0.year,
            &input.0.standings_type,
            &self.http,
            &self.cache,
            jolpica,
        )
        .await
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

    let config: MotorsportConfig = if let Some(path) = &cli.config {
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

    let server = MotorsportServer::new(config, http, cache);

    tracing::info!("starting mcp-motorsport server (stdio transport)");
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;

    Ok(())
}

const DEFAULT_CONFIG: &str = r#"
transport = "stdio"

[cache]
max_entries = 10000
ttl_seconds = 3600

[openf1]
base_url = "https://api.openf1.org/v1"
rate_limit_rps = 3.0

[jolpica]
base_url = "https://api.jolpi.ca/ergast/f1"
"#;
