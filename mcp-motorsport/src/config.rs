//! Motorsport server configuration.

use mcp_common::ServerConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MotorsportConfig {
    #[serde(flatten)]
    pub server: ServerConfig,
    pub openf1: OpenF1Config,
    pub jolpica: Option<JolpicaConfig>,
}

#[derive(Debug, Deserialize)]
pub struct OpenF1Config {
    pub base_url: String,
    pub rate_limit_rps: f64,
    pub auth_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JolpicaConfig {
    pub base_url: String,
}
