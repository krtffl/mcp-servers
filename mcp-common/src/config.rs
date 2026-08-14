//! Base configuration types shared across all MCP servers.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

/// Base server configuration common to all MCP servers.
#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    /// Transport mode: stdio or sse.
    #[serde(default)]
    pub transport: TransportMode,

    /// Bind address for SSE transport (ignored in stdio mode).
    pub listen_addr: Option<String>,

    /// Cache configuration.
    #[serde(default)]
    pub cache: CacheConfig,

    /// Log level (trace, debug, info, warn, error).
    pub log_level: Option<String>,
}

/// Transport mode for MCP communication.
#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransportMode {
    #[default]
    Stdio,
    Sse,
}

/// Cache configuration.
#[derive(Debug, Deserialize)]
pub struct CacheConfig {
    /// Maximum number of cached entries.
    #[serde(default = "default_max_entries")]
    pub max_entries: u64,

    /// Default TTL in seconds for cached responses.
    #[serde(default = "default_ttl_seconds")]
    pub ttl_seconds: u64,

    /// Per-tool TTL overrides (`tool_name` → seconds).
    pub tool_ttls: Option<HashMap<String, u64>>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: default_max_entries(),
            ttl_seconds: default_ttl_seconds(),
            tool_ttls: None,
        }
    }
}

const fn default_max_entries() -> u64 {
    1000
}

const fn default_ttl_seconds() -> u64 {
    60
}

/// Load and parse a TOML configuration file.
///
/// # Errors
///
/// Returns `McpServerError::Config` if the file cannot be read or parsed.
pub fn load_config<T: serde::de::DeserializeOwned>(
    path: &Path,
) -> Result<T, crate::McpServerError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        crate::McpServerError::Config(format!("failed to read config {}: {e}", path.display()))
    })?;

    toml::from_str(&content).map_err(|e| {
        crate::McpServerError::Config(format!("failed to parse config {}: {e}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cache_config() {
        let config = CacheConfig::default();
        assert_eq!(config.max_entries, 1000);
        assert_eq!(config.ttl_seconds, 60);
        assert!(config.tool_ttls.is_none());
    }

    #[test]
    fn deserialize_transport_mode() {
        #[derive(Deserialize)]
        struct Wrapper {
            mode: TransportMode,
        }
        let stdio: Wrapper = toml::from_str("mode = \"stdio\"").unwrap();
        assert_eq!(stdio.mode, TransportMode::Stdio);

        let sse: Wrapper = toml::from_str("mode = \"sse\"").unwrap();
        assert_eq!(sse.mode, TransportMode::Sse);
    }

    #[test]
    fn deserialize_server_config() {
        let toml_str = r#"
transport = "stdio"
log_level = "info"

[cache]
max_entries = 500
ttl_seconds = 120
"#;
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.transport, TransportMode::Stdio);
        assert_eq!(config.log_level.as_deref(), Some("info"));
        assert_eq!(config.cache.max_entries, 500);
        assert_eq!(config.cache.ttl_seconds, 120);
    }
}
