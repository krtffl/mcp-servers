# MCP Servers

Three Rust MCP servers in a Cargo workspace monorepo, built on rmcp SDK v1.2.0.
Infrastructure metrics, Spanish government data, and F1 motorsport analytics.

## Commands

| Command | Description |
|---------|-------------|
| `cargo build` | Build all workspace crates |
| `cargo test` | Run all tests |
| `cargo clippy --all-targets -- -D warnings` | Lint with pedantic warnings as errors |
| `cargo fmt --check` | Check formatting |
| `cargo run -p mcp-infra` | Run infrastructure server (stdio) |
| `cargo run -p mcp-spain` | Run Spanish data server (stdio) |
| `cargo run -p mcp-motorsport` | Run motorsport server (stdio) |

## Architecture

```
mcp-servers/
├── mcp-common/        # Shared: cache, config, errors, HTTP client, rate limiter
├── mcp-infra/         # Infrastructure: Prometheus, Docker, health, Grafana, alerts
├── mcp-spain/         # Spanish data: BOE, CNAE, AEAT calendar, Catastro, Verifactu
└── mcp-motorsport/    # F1 data: race results, lap times, telemetry, stints, standings
```

## Key Patterns

- **rmcp macros**: `#[tool_router]`, `#[tool]`, `#[tool_handler]` for tool registration
- **Tool return type**: `Result<String, String>` (JSON on success, error message on failure)
- **Parameters**: `Parameters<T>` wrapper with `schemars::JsonSchema` for auto schema
- **Shared state**: Server struct holds `Arc<Config>`, `reqwest::Client`, `Arc<ResponseCache>`
- **Error handling**: `thiserror` in mcp-common, `anyhow` in binaries
- **Logging**: `tracing` to stderr (stdout reserved for MCP protocol)
- **No `unwrap()` or `expect()`** in library code
- **Edition 2024** with `clippy::pedantic` as baseline
- **Conventional commits**: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `ci:`

## Cross-references

- Product definition: /home/krtffl/Documents/product-portfolio/products/P2-mcp-servers.md
- Tech specification: /home/krtffl/Documents/product-portfolio/products/P2-mcp-servers-tech.md
- Portfolio plan: /home/krtffl/Documents/product-portfolio/04-execution-plan.md
