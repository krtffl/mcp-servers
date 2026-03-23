//! mcp-infra: MCP server for self-hosted infrastructure metrics.
//!
//! Tools: get_server_health, list_docker_containers, query_prometheus,
//! get_grafana_dashboard_url, list_recent_alerts.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use mcp_common::{ResponseCache, build_http_client};
use mcp_infra::config::InfraConfig;
use mcp_infra::tools;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{ServerHandler, ServiceExt as _, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(name = "mcp-infra", version, about = "MCP server for infrastructure metrics")]
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
pub struct GetServerHealthInput {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListContainersInput {
    /// Filter containers by name substring.
    pub name_filter: Option<String>,
    /// Include stopped containers (default: false).
    pub all: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryPrometheusInput {
    /// PromQL query expression.
    pub query: String,
    /// Evaluation timestamp (RFC3339 or Unix). Defaults to now.
    pub time: Option<String>,
    /// Range query start time.
    pub start: Option<String>,
    /// Range query end time.
    pub end: Option<String>,
    /// Range query step interval (e.g., 15s, 1m, 5m).
    pub step: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetGrafanaDashboardInput {
    /// Dashboard name to search for.
    pub name: Option<String>,
    /// Dashboard UID for direct lookup.
    pub uid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListAlertsInput {
    /// Filter by alert status (firing, resolved).
    pub status: Option<String>,
    /// Filter by severity (critical, warning, info).
    pub severity: Option<String>,
}

// --- Server ---

#[derive(Clone)]
pub struct InfraServer {
    tool_router: ToolRouter<Self>,
    config: Arc<InfraConfig>,
    http: reqwest::Client,
    cache: Arc<ResponseCache>,
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for InfraServer {}

#[tool_router(router = tool_router)]
impl InfraServer {
    pub fn new(config: InfraConfig, http: reqwest::Client, cache: ResponseCache) -> Self {
        Self {
            tool_router: Self::tool_router(),
            config: Arc::new(config),
            http,
            cache: Arc::new(cache),
        }
    }

    #[tool(
        name = "get_server_health",
        description = "Get server health metrics: CPU, memory, disk usage, network I/O, uptime"
    )]
    async fn get_server_health(
        &self,
        _input: Parameters<GetServerHealthInput>,
    ) -> Result<String, String> {
        tools::health::execute()
    }

    #[tool(
        name = "list_docker_containers",
        description = "List Docker containers with status, image, and port mappings"
    )]
    async fn list_docker_containers(
        &self,
        input: Parameters<ListContainersInput>,
    ) -> Result<String, String> {
        tools::docker::execute(input.0.name_filter.as_deref(), input.0.all.unwrap_or(false))
            .await
    }

    #[tool(
        name = "query_prometheus",
        description = "Query Prometheus metrics using PromQL. Supports instant and range queries."
    )]
    async fn query_prometheus(
        &self,
        input: Parameters<QueryPrometheusInput>,
    ) -> Result<String, String> {
        tools::prometheus::execute(
            &input.0.query,
            input.0.time.as_deref(),
            input.0.start.as_deref(),
            input.0.end.as_deref(),
            input.0.step.as_deref(),
            &self.http,
            &self.cache,
            &self.config.prometheus,
        )
        .await
    }

    #[tool(
        name = "get_grafana_dashboard_url",
        description = "Get Grafana dashboard URLs by name or UID"
    )]
    async fn get_grafana_dashboard_url(
        &self,
        input: Parameters<GetGrafanaDashboardInput>,
    ) -> Result<String, String> {
        let grafana = self
            .config
            .grafana
            .as_ref()
            .ok_or("Grafana is not configured. Add a [grafana] section to your config file.")?;
        tools::grafana::execute(
            input.0.name.as_deref(),
            input.0.uid.as_deref(),
            &self.http,
            &self.cache,
            grafana,
        )
        .await
    }

    #[tool(
        name = "list_recent_alerts",
        description = "List recent Alertmanager alerts, optionally filtered by status or severity"
    )]
    async fn list_recent_alerts(
        &self,
        input: Parameters<ListAlertsInput>,
    ) -> Result<String, String> {
        let alertmanager = self.config.alertmanager.as_ref().ok_or(
            "Alertmanager is not configured. Add an [alertmanager] section to your config file.",
        )?;
        tools::alerts::execute(
            input.0.status.as_deref(),
            input.0.severity.as_deref(),
            &self.http,
            &self.cache,
            alertmanager,
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

    let config: InfraConfig = if let Some(path) = &cli.config {
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

    let server = InfraServer::new(config, http, cache);

    tracing::info!("starting mcp-infra server (stdio transport)");
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;

    Ok(())
}

const DEFAULT_CONFIG: &str = r#"
transport = "stdio"

[cache]
max_entries = 1000
ttl_seconds = 60

[prometheus]
url = "http://localhost:9090"

[docker]
"#;
