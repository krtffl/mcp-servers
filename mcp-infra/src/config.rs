//! Infrastructure server configuration.

use mcp_common::ServerConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct InfraConfig {
    #[serde(flatten)]
    pub server: ServerConfig,
    pub prometheus: PrometheusConfig,
    #[serde(default)]
    pub docker: DockerConfig,
    pub grafana: Option<GrafanaConfig>,
    pub alertmanager: Option<AlertmanagerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct PrometheusConfig {
    pub url: String,
    pub auth_token: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct DockerConfig {
    pub socket: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GrafanaConfig {
    pub url: String,
    pub auth_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AlertmanagerConfig {
    pub url: String,
}
