mod state;

use mcp_core::error::SearchOptions;
use mcp_transport::SseTransportConfig;
use rmcp::{
    tool_router, tool,
    handler::server::wrapper::{Parameters, Json},
    schemars,
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::EnvFilter;

use state::{AppState, ProxyConfig};

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchInput {
    query: String,
    #[serde(default)]
    engine: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    region: Option<String>,
    #[serde(default = "default_page_size")]
    page_size: i64,
    #[serde(default)]
    page: Option<i64>,
}

fn default_page_size() -> i64 {
    10
}

#[derive(Serialize, schemars::JsonSchema)]
struct SearchOutput {
    query: String,
    results: Vec<SearchResultItem>,
    total_results: i64,
    engine: String,
    search_time_ms: i64,
}

#[derive(Serialize, schemars::JsonSchema)]
struct SearchResultItem {
    title: String,
    url: String,
    snippet: String,
    engine: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    published_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<i64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct MultiSearchInput {
    queries: Vec<QueryItem>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    region: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct QueryItem {
    engine: String,
    query: String,
}

#[derive(Serialize, schemars::JsonSchema)]
struct MultiSearchOutput {
    responses: Vec<SearchOutput>,
    total_results: i64,
    search_time_ms: i64,
}

#[derive(Serialize, schemars::JsonSchema)]
struct EnginesOutput {
    engines: Vec<EngineInfo>,
}

#[derive(Serialize, schemars::JsonSchema)]
struct EngineInfo {
    name: String,
    available: bool,
}

struct McpServer {
    state: Arc<AppState>,
}

#[tool_router(server_handler)]
impl McpServer {
    #[tool(name = "web_search", description = "Search the web using multiple search engines. Returns unified results from Google, DuckDuckGo, Bing, Brave, YouTube, or Yahoo.")]
    async fn search(
        &self,
        Parameters(input): Parameters<SearchInput>,
    ) -> Json<SearchOutput> {
        let state = Arc::clone(&self.state);
        let query = input.query.clone();
        let engine = input.engine.clone();

        let options = SearchOptions {
            language: input.language,
            region: input.region,
            page_size: input.page_size as usize,
            page: input.page.map(|p| p as usize),
            safe_search: None,
            time_range: None,
        };

        let result = state.search(&query, engine.as_deref(), &options).await;

        match result {
            Ok(response) => Json(SearchOutput {
                query: response.query,
                results: response.results.into_iter().map(|r| SearchResultItem {
                    title: r.title,
                    url: r.url,
                    snippet: r.snippet,
                    engine: r.engine,
                    published_date: r.published_date,
                    position: r.position.map(|p| p as i64),
                }).collect(),
                total_results: response.total_results.unwrap_or(0) as i64,
                engine: response.engine,
                search_time_ms: response.search_time_ms as i64,
            }),
            Err(_e) => Json(SearchOutput {
                query: query,
                results: vec![],
                total_results: 0,
                engine: engine.unwrap_or_else(|| "unknown".to_string()),
                search_time_ms: 0,
            }),
        }
    }

    #[tool(name = "multi_search", description = "Search across multiple engines simultaneously. Returns aggregated results from all specified engines.")]
    async fn multi_search(
        &self,
        Parameters(input): Parameters<MultiSearchInput>,
    ) -> Json<MultiSearchOutput> {
        let state = Arc::clone(&self.state);

        let queries: Vec<(String, String)> = input.queries.iter()
            .map(|q| (q.engine.clone(), q.query.clone()))
            .collect();

        let options = SearchOptions {
            language: input.language,
            region: input.region,
            page_size: 10,
            page: None,
            safe_search: None,
            time_range: None,
        };

        let result = state.aggregator.search_multi(&queries, &options).await;

        let responses: Vec<SearchOutput> = result.into_iter()
            .filter_map(|r| r.ok())
            .map(|r| SearchOutput {
                query: r.query,
                results: r.results.into_iter().map(|res| SearchResultItem {
                    title: res.title,
                    url: res.url,
                    snippet: res.snippet,
                    engine: res.engine,
                    published_date: res.published_date,
                    position: res.position.map(|p| p as i64),
                }).collect(),
                total_results: r.total_results.unwrap_or(0) as i64,
                engine: r.engine,
                search_time_ms: r.search_time_ms as i64,
            })
            .collect();

        let total_results: i64 = responses.iter().map(|r| r.total_results).sum();

        Json(MultiSearchOutput {
            responses,
            total_results,
            search_time_ms: 0,
        })
    }

    #[tool(name = "list_engines", description = "List all available search engines and their status.")]
    fn list_engines(&self) -> Json<EnginesOutput> {
        let engines: Vec<EngineInfo> = self.state.aggregator.engine_names()
            .into_iter()
            .map(|name| EngineInfo {
                name: name.to_string(),
                available: true,
            })
            .collect();

        Json(EnginesOutput { engines })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Starting MCP Web Search Server v{}", env!("CARGO_PKG_VERSION"));

    // Parse CLI arguments
    let args: Vec<String> = std::env::args().collect();

    // Check for --http flag to run HTTP/SSE server
    let http_mode = args.contains(&"--http".to_string());
    let http_port = args.iter()
        .position(|a| a == "--port")
        .and_then(|i| args.get(i + 1))
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);
    let http_host = args.iter()
        .position(|a| a == "--host")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "127.0.0.1".to_string());

    // Check for proxy configuration
    let proxy_config = if let Some(proxy_arg) = args.iter().position(|a| a == "--proxy") {
        let proxy_url = args.get(proxy_arg + 1).cloned().unwrap_or_default();
        ProxyConfig {
            http_proxy: Some(proxy_url.clone()),
            https_proxy: Some(proxy_url),
            ..Default::default()
        }
    } else {
        ProxyConfig::from_env()
    };

    // Check for plugin directory
    let plugin_dir = args.iter()
        .position(|a| a == "--plugins")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .or_else(|| {
            let default_dir = PathBuf::from("plugins");
            if default_dir.exists() {
                Some(default_dir)
            } else {
                None
            }
        });

    let state = Arc::new(AppState::with_config(Some(proxy_config), plugin_dir.as_ref()));
    let server = McpServer { state };

    if http_mode {
        // Run HTTP/SSE server
        let sse_config = SseTransportConfig {
            host: http_host,
            port: http_port,
        };
        info!("Running in HTTP/SSE mode on {}:{}", sse_config.host, sse_config.port);
        mcp_transport::run_sse_server(sse_config).await?;
    } else {
        // Run stdio server (default)
        let transport = (
            tokio::io::BufReader::new(tokio::io::stdin()),
            tokio::io::BufWriter::new(tokio::io::stdout()),
        );

        let service = rmcp::serve_server(server, transport).await?;

        info!("MCP server running on stdio");
        service.waiting().await?;
    }

    Ok(())
}
