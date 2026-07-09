<div align="center">

# MCP Web Search

**Enterprise-grade MCP server for unified web search across multiple search engines**

[![CI](https://github.com/vitkuz573/mcp-web-search/actions/workflows/ci.yml/badge.svg)](https://github.com/vitkuz573/mcp-web-search/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024-orange.svg)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-2.2-green.svg)](https://modelcontextprotocol.io/)

[Features](#features) · [Quick Start](#quick-start) · [Configuration](#configuration) · [Docker](#docker) · [API](#api) · [Architecture](#architecture) · [Contributing](#contributing)

</div>

---

## Overview

MCP Web Search is a high-performance Rust server that implements the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) to provide unified web search capabilities across multiple search engines. It parses search engine HTML pages directly — no paid API keys required.

### Supported Engines

| Engine | Status | Results per query |
|--------|--------|-------------------|
| Google | Available* | 10 |
| Bing | Available | 10 |
| Brave | Available | 20 |
| DuckDuckGo | Available* | 10 |
| Yahoo | Available | ~7 |
| YouTube | Available* | varies |

> \* May be rate-limited or CAPTCHA-blocked from server IPs. Works reliably from residential connections.

---

## Features

- **6 Search Engines** — Google, Bing, Brave, DuckDuckGo, Yahoo, YouTube
- **MCP Protocol** — Full MCP 2.2 support via stdio transport
- **No API Keys** — Parses HTML directly, no paid APIs needed
- **Async & Fast** — Built on Tokio with concurrent multi-engine search
- **Smart Caching** — Built-in response cache (moka) with 1-hour TTL
- **Docker Ready** — Multi-stage Dockerfile with minimal runtime image (~90MB)
- **Type Safe** — Full Rust type system with comprehensive error handling
- **Structured Logging** — Tracing-based logging with configurable levels

---

## Quick Start

### Prerequisites

- Rust 1.85+ (edition 2024)
- `cargo`

### Build & Run

```bash
git clone https://github.com/vitkuz573/mcp-web-search.git
cd mcp-web-search
cargo build --release

# Run the MCP server
./target/release/mcp-web-search
```

### Configure for OpenCode

Add to `~/.config/opencode/opencode.jsonc`:

```jsonc
{
  "mcp": {
    "web-search": {
      "type": "local",
      "command": ["/path/to/mcp-web-search"],
      "enabled": true,
      "environment": {
        "RUST_LOG": "off"
      }
    }
  }
}
```

### Available Tools

Once connected, the following MCP tools are available:

#### `web_search`

Search a single engine or the default engine.

```json
{
  "query": "rust programming language",
  "engine": "bing",
  "language": "en",
  "region": "us",
  "page_size": 10,
  "page": 1
}
```

#### `multi_search`

Search across multiple engines simultaneously.

```json
{
  "queries": [
    { "engine": "bing", "query": "rust lang" },
    { "engine": "brave", "query": "rust lang" }
  ],
  "language": "en",
  "region": "us"
}
```

#### `list_engines`

List all available search engines and their status.

```json
{}
```

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level (`off`, `error`, `warn`, `info`, `debug`, `trace`) |

### MCP Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `type` | string | required | Must be `"local"` |
| `command` | array | required | Command to start the server |
| `enabled` | bool | `true` | Enable/disable the server |
| `timeout` | number | `5000` | Timeout in ms for tool fetching |
| `environment` | object | `{}` | Environment variables |

---

## Docker

### Build & Run

```bash
# Build the image
docker build -t mcp-web-search .

# Run via docker compose
docker compose run --rm mcp-web-search

# Or run directly
docker run -it --rm mcp-web-search
```

### Docker Compose

```yaml
services:
  mcp-web-search:
    build: .
    stdin_open: true
    tty: false
    environment:
      - RUST_LOG=info
    restart: unless-stopped
```

The server communicates via **stdio** (stdin/stdout), which is the standard MCP transport.

---

## Architecture

```
mcp-web-search/
├── crates/
│   ├── mcp-core/        # Core traits, types, error handling
│   ├── mcp-parser/      # HTML parsing for search engine results
│   ├── mcp-search/      # Search engine implementations
│   ├── mcp-transport/   # SSE + Stdio transport layer
│   └── mcp-web-search/  # Main binary (MCP server)
├── Dockerfile           # Multi-stage Docker build
├── docker-compose.yml   # Docker Compose config
└── Cargo.toml           # Workspace root
```

### Crate Overview

| Crate | Description |
|-------|-------------|
| `mcp-core` | Core traits (`SearchEngine`), types (`SearchResult`, `SearchResponse`), and error handling |
| `mcp-parser` | HTML parsing utilities for extracting search results from engine pages |
| `mcp-search` | Search engine implementations with plugin architecture |
| `mcp-transport` | SSE + Stdio transport layer (MCP protocol) |
| `mcp-web-search` | Main binary — MCP server with `web_search`, `multi_search`, `list_engines` tools |

### Search Flow

```
Client → MCP Protocol → mcp-web-search
                            ↓
                     SearchAggregator
                            ↓
              ┌─────────────┼─────────────┐
              ↓             ↓             ↓
           Bing          Brave        Yahoo
              ↓             ↓             ↓
           Parser        Parser        Parser
              ↓             ↓             ↓
              └─────────────┼─────────────┘
                            ↓
                      SearchResponse
                            ↓
                     MCP Response → Client
```

---

## API Reference

### MCP Tools

#### `web_search`

```rust
struct SearchInput {
    query: String,
    engine: Option<String>,      // "bing", "brave", "google", etc.
    language: Option<String>,    // "en", "ru", "de", etc.
    region: Option<String>,      // "us", "ru", etc.
    page_size: i64,              // default: 10
    page: Option<i64>,
}
```

#### `multi_search`

```rust
struct MultiSearchInput {
    queries: Vec<QueryItem>,     // max concurrent searches
    language: Option<String>,
    region: Option<String>,
}

struct QueryItem {
    engine: String,
    query: String,
}
```

#### `list_engines`

```rust
struct EnginesOutput {
    engines: Vec<EngineInfo>,
}

struct EngineInfo {
    name: String,
    available: bool,
}
```

---

## Development

### Prerequisites

- Rust 1.85+
- `cargo`

### Build

```bash
cargo build --workspace
```

### Test

```bash
cargo test --workspace
```

### Lint

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

### Project Structure

Each crate is independently testable:

```bash
# Test specific crate
cargo test -p mcp-parser
cargo test -p mcp-search
```

---

## Roadmap

- [ ] Retry logic with exponential backoff for rate-limited engines
- [ ] Configurable per-engine result limits
- [ ] Proxy support for geo-restricted searches
- [ ] Search result caching with Redis backend
- [ ] HTTP/SSE transport option (alongside stdio)
- [ ] Custom engine plugin API
- [ ] Rate limiting per-client
- [ ] Metrics export (Prometheus)

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

---

## Author

**Vitaly Kuzyaev** — [vitkuz573@gmail.com](mailto:vitkuz573@gmail.com)

---

<div align="center">

Built with Rust 🦀 and the [Model Context Protocol](https://modelcontextprotocol.io/)

</div>
