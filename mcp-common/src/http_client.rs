//! Shared HTTP client builder with connection pooling and timeouts.

use std::time::Duration;

/// Build a shared `reqwest::Client` with sensible defaults for MCP server use.
///
/// - 30-second timeout for all requests
/// - Connection pooling enabled (default idle pool size)
/// - rustls TLS backend (no OpenSSL dependency)
///
/// # Errors
///
/// Returns `McpServerError::Config` if the client cannot be built.
pub fn build_http_client() -> Result<reqwest::Client, crate::McpServerError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(5)
        .user_agent(concat!("mcp-servers/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| crate::McpServerError::Config(format!("failed to build HTTP client: {e}")))
}
