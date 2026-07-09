<div align="center">

# MCP Web Search

**Enterprise-grade MCP server for unified web search across multiple search engines**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024-orange.svg)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-2.2-green.svg)](https://modelcontextprotocol.io/)

[Features](#features) · [Quick Start](#quick-start) · [Configuration](#configuration) · [Plugins](#custom-engine-plugins) · [Proxy](#proxy-support) · [Docker](#docker) · [Architecture](#architecture)

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
| Custom | TOML plugins | configurable |

> \* May be rate-limited or CAPTCHA-blocked from server IPs. Works reliably from residential connections.

---

## Features

- **6 Built-in Engines** — Google, Bing, Brave, DuckDuckGo, Yahoo, YouTube
- **Custom Engine Plugins** — Add any search engine via TOML config files
- **MCP Protocol** — Full MCP 2.2 support via stdio and HTTP/SSE transports
- **No API Keys** — Parses HTML directly, no paid APIs needed
- **Proxy Support** — HTTP, HTTPS, SOCKS5 with per-engine proxy routing
- **Async & Fast** — Built on Tokio with concurrent multi-engine search
- **Smart Caching** — Built-in response cache (moka) with 1-hour TTL
- **Docker Ready** — Multi-stage Dockerfile with minimal runtime image (~90MB)
- **Type Safe** — Full Rust type system with comprehensive error handling

---

## Quick Start

### Prerequisites

- Rust 1.85+ (edition 2024)

### Build & Run

```bash
git clone https://github.com/vitkuz573/mcp-web-search.git
cd mcp-web-search
cargo build --release

# Run via stdio (default — for MCP clients like OpenCode)
./target/release/mcp-web-search

# Run via HTTP/SSE (for remote access)
./target/release/mcp-web-search --http --port 3000

# Run with proxy
HTTP_PROXY=http://proxy:8080 ./target/release/mcp-web-search

# Run with custom plugins directory
./target/release/mcp-web-search --plugins ./my-plugins
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
      "timeout": 10000,
      "environment": {
        "RUST_LOG": "off"
      }
    }
  }
}
```

---

## Custom Engine Plugins

Add any search engine as a TOML plugin file in the `plugins/` directory.

### Plugin Format

```toml
name = "startpage"
description = "Startpage privacy-focused search engine"
base_url = "https://www.startpage.com"
search_url_template = "https://www.startpage.com/sp/search?query={query}&language={lang}&cat=web"
result_container = "div.w-gl__result"
title_selector = "h3.w-gl__result-title"
url_selector = "a.w-gl__result-url"
snippet_selector = "p.w-gl__description"
date_selector = "span.w-gl__date"
next_page_selector = "button.next"
url_attr = "href"
timeout_ms = 15000

[headers]
Accept = "text/html,application/xhtml+xml"
DNT = "1"
```

### Placeholders

| Placeholder | Description |
|-------------|-------------|
| `{query}` | URL-encoded search query |
| `{lang}` | Language code (e.g., `en`, `de`) |
| `{region}` | Region code (e.g., `us`, `at`) |
| `{page}` | Page number (0-indexed) |
| `{page_size}` | Results per page |
| `{first}` | First result index (1-indexed) |

### Load Plugins

```bash
# Auto-load from ./plugins/ directory
./target/release/mcp-web-search

# Specify custom directory
./target/release/mcp-web-search --plugins /path/to/plugins

# Load single file
./target/release/mcp-web-search --plugins ./my-engine.toml
```

---

## Proxy Support

Route HTTP requests through proxies. Supports HTTP, HTTPS, and SOCKS5.

### Environment Variables

```bash
# HTTP proxy
HTTP_PROXY=http://proxy:8080 ./target/release/mcp-web-search

# HTTPS proxy
HTTPS_PROXY=http://proxy:8080 ./target/release/mcp-web-search

# SOCKS5 proxy
ALL_PROXY=socks5://proxy:1080 ./target/release/mcp-web-search

# No proxy for specific hosts
NO_PROXY=localhost,127.0.0.1 ./target/release/mcp-web-search
```

### CLI Flag

```bash
./target/release/mcp-web-search --proxy http://proxy:8080
```

### Per-Engine Proxy

Use TOML config for per-engine proxy routing:

```toml
http_proxy = "http://us-proxy:8080"
https_proxy = "http://us-proxy:8080"
socks5_proxy = "socks5://eu-proxy:1080"
no_proxy = "localhost,127.0.0.1"

[engine_proxies]
bing = "http://bing-proxy:8080"
brave = "http://brave-proxy:8080"
```

---

## Configuration

### CLI Options

| Flag | Description |
|------|-------------|
| `--http` | Run in HTTP/SSE mode (default: stdio) |
| `--port <PORT>` | HTTP server port (default: 3000) |
| `--host <HOST>` | HTTP server host (default: 127.0.0.1) |
| `--proxy <URL>` | Global proxy URL |
| `--plugins <PATH>` | Plugin file or directory path |

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level (`off`, `error`, `warn`, `info`, `debug`, `trace`) |
| `HTTP_PROXY` | — | HTTP proxy URL |
| `HTTPS_PROXY` | — | HTTPS proxy URL |
| `ALL_PROXY` | — | SOCKS5 proxy URL |
| `NO_PROXY` | — | Comma-separated no-proxy list |

### MCP Configuration

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
      - HTTP_PROXY=http://proxy:8080
    restart: unless-stopped
```

---

## Architecture

```
mcp-web-search/
├── crates/
│   ├── mcp-core/        # Core traits, types, error handling
│   ├── mcp-parser/      # HTML parsing for search engine results
│   ├── mcp-search/      # Search engine implementations + custom plugins
│   ├── mcp-transport/   # SSE + HTTP + Stdio transport layer
│   └── mcp-web-search/  # Main binary (MCP server)
├── plugins/             # Custom engine TOML plugins
├── Dockerfile           # Multi-stage Docker build
├── docker-compose.yml   # Docker Compose config
└── Cargo.toml           # Workspace root
```

### Crate Overview

| Crate | Description |
|-------|-------------|
| `mcp-core` | Core traits (`SearchEngine`), types, error handling |
| `mcp-parser` | HTML parsing utilities with CSS selectors |
| `mcp-search` | Search engine implementations + custom plugin loader |
| `mcp-transport` | SSE, HTTP, Stdio transport (MCP protocol) |
| `mcp-web-search` | Main binary — MCP server with all tools |

### MCP Tools

#### `web_search`

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

```json
{}
```

---

## Development

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

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

---

## Author

**Vitaly Kuzyaev** — [vitkuz573@gmail.com](mailto:vitkuz573@gmail.com)

---

<div align="center">

Built with Rust and the [Model Context Protocol](https://modelcontextprotocol.io/)

</div>
