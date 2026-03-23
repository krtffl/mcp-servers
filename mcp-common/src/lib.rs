//! Shared utilities for MCP server crates.
//!
//! Provides caching, rate limiting, configuration, HTTP client building,
//! and unified error types used across all three MCP servers.

pub mod cache;
pub mod config;
pub mod error;
pub mod http_client;
pub mod rate_limit;

pub use cache::ResponseCache;
pub use config::{CacheConfig, ServerConfig, TransportMode};
pub use error::McpServerError;
pub use http_client::build_http_client;
pub use rate_limit::RateLimiter;
