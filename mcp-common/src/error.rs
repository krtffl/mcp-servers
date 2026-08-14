//! Unified error types for MCP server crates.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum McpServerError {
    #[error("external API request failed: {url} — {reason}")]
    ExternalApi { url: String, reason: String },

    #[error("unexpected response from {url}: {reason}")]
    UnexpectedResponse { url: String, reason: String },

    #[error("rate limit exceeded for {api}, retry after {retry_after_secs}s")]
    RateLimited { api: String, retry_after_secs: u64 },

    #[error("invalid tool input: {0}")]
    InvalidInput(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("docker API error: {0}")]
    Docker(String),

    #[error("XML parse error: {0}")]
    XmlParse(String),

    #[error("HTML parse error: selector `{selector}` — {reason}")]
    HtmlParse { selector: String, reason: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("resource not configured: {0}")]
    NotConfigured(String),
}
