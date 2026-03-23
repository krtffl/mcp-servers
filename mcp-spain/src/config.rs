//! Spain server configuration.

use mcp_common::ServerConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SpainConfig {
    #[serde(flatten)]
    pub server: ServerConfig,
    pub boe: BoeConfig,
    pub aeat: AeatConfig,
    pub cnae: CnaeConfig,
    pub catastro: CatastroConfig,
    pub verifactu: VerifactuConfig,
}

#[derive(Debug, Deserialize)]
pub struct BoeConfig {
    pub base_url: String,
    pub rate_limit_rps: f64,
}

#[derive(Debug, Deserialize)]
pub struct AeatConfig {
    pub base_url: String,
    pub rate_limit_rps: f64,
}

#[derive(Debug, Deserialize)]
pub struct CnaeConfig {
    pub base_url: String,
    pub fallback_csv: Option<String>,
    pub rate_limit_rps: f64,
}

#[derive(Debug, Deserialize)]
pub struct CatastroConfig {
    pub base_url: String,
    pub rate_limit_rps: f64,
}

#[derive(Debug, Deserialize)]
pub struct VerifactuConfig {
    pub base_url: String,
    pub rate_limit_rps: f64,
}
