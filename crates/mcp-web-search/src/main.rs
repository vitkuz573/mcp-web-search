mod state;
mod analytics;

use analytics::SearchStatsSnapshot;

use mcp_core::error::{SearchOptions, TimeRange};
use mcp_transport::SseTransportConfig;
use rmcp::{
    tool_router, tool,
    handler::server::wrapper::{Parameters, Json},
    schemars,
};
use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::path::PathBuf;
use std::time::Instant;
use tracing::info;
use tracing_subscriber::EnvFilter;

use state::{AppState, ProxyConfig};

// ═══════════════════════════════════════════════════════════════════
// MCP Tool Schemas
// ═══════════════════════════════════════════════════════════════════

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
    /// Filter by time range: "day", "week", "month", "year"
    #[serde(default)]
    time_range: Option<String>,
    /// Safe search filtering: true enables adult content filtering
    #[serde(default)]
    safe_search: Option<bool>,
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
    /// Filter by time range: "day", "week", "month", "year"
    #[serde(default)]
    time_range: Option<String>,
    /// Safe search filtering
    #[serde(default)]
    safe_search: Option<bool>,
    /// Remove duplicate results across engines (default: true)
    #[serde(default = "default_true")]
    dedup: bool,
}

fn default_true() -> bool {
    true
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
    duplicates_removed: i64,
}

#[derive(Serialize, schemars::JsonSchema)]
struct EnginesOutput {
    engines: Vec<EngineInfo>,
}

#[derive(Serialize, schemars::JsonSchema)]
struct EngineInfo {
    name: String,
    available: bool,
    avg_latency_ms: f64,
    success_rate: f64,
    total_searches: i64,
}

#[derive(Serialize, schemars::JsonSchema)]
struct HealthCheckOutput {
    engines: Vec<EngineHealth>,
    total_time_ms: i64,
}

#[derive(Serialize, schemars::JsonSchema)]
struct EngineHealth {
    name: String,
    status: String,
    latency_ms: i64,
    results_count: i64,
    error: Option<String>,
}

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
struct ExportInput {
    query: String,
    #[serde(default)]
    engine: Option<String>,
    /// Export format: "json", "csv", "markdown"
    #[serde(default = "default_export_format")]
    format: String,
}

fn default_export_format() -> String {
    "json".to_string()
}

#[derive(Serialize, schemars::JsonSchema)]
struct ExportOutput {
    content: String,
    format: String,
    result_count: i64,
}

// ═══════════════════════════════════════════════════════════════════
// MCP Server
// ═══════════════════════════════════════════════════════════════════

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

        let time_range = input.time_range.as_deref().and_then(parse_time_range);

        let options = SearchOptions {
            language: input.language,
            region: input.region,
            page_size: input.page_size as usize,
            page: input.page.map(|p| p as usize),
            safe_search: input.safe_search,
            time_range,
        };

        let result = state.search(&query, engine.as_deref(), &options).await;

        match result {
            Ok(response) => {
                state.analytics.record_search(&response.engine, response.search_time_ms, response.results.len());
                Json(SearchOutput {
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
                })
            }
            Err(_e) => Json(SearchOutput {
                query: query,
                results: vec![],
                total_results: 0,
                engine: engine.unwrap_or_else(|| "unknown".to_string()),
                search_time_ms: 0,
            }),
        }
    }

    #[tool(name = "multi_search", description = "Search across multiple engines simultaneously. Returns aggregated and deduplicated results from all specified engines.")]
    async fn multi_search(
        &self,
        Parameters(input): Parameters<MultiSearchInput>,
    ) -> Json<MultiSearchOutput> {
        let state = Arc::clone(&self.state);
        let start = Instant::now();

        let queries: Vec<(String, String)> = input.queries.iter()
            .map(|q| (q.engine.clone(), q.query.clone()))
            .collect();

        let time_range = input.time_range.as_deref().and_then(parse_time_range);

        let options = SearchOptions {
            language: input.language,
            region: input.region,
            page_size: 10,
            page: None,
            safe_search: input.safe_search,
            time_range,
        };

        let result = state.aggregator.search_multi(&queries, &options).await;

        let mut all_results: Vec<SearchResultItem> = result.into_iter()
            .filter_map(|r| r.ok())
            .flat_map(|r| {
                state.analytics.record_search(&r.engine, r.search_time_ms, r.results.len());
                r.results.into_iter().map(move |res| SearchResultItem {
                    title: res.title,
                    url: res.url,
                    snippet: res.snippet,
                    engine: res.engine,
                    published_date: res.published_date,
                    position: res.position.map(|p| p as i64),
                })
            })
            .collect();

        // Deduplicate by URL
        let mut seen_urls: HashSet<String> = HashSet::new();
        let original_count = all_results.len() as i64;

        if input.dedup {
            all_results.retain(|r| seen_urls.insert(r.url.clone()));
        }

        let duplicates_removed = original_count - all_results.len() as i64;
        state.analytics.record_dedup(duplicates_removed);

        let total_results = all_results.len() as i64;

        // Re-number positions after dedup
        for (i, r) in all_results.iter_mut().enumerate() {
            r.position = Some(i as i64 + 1);
        }

        let search_time = start.elapsed().as_millis() as i64;

        Json(MultiSearchOutput {
            responses: vec![SearchOutput {
                query: input.queries.first().map(|q| q.query.clone()).unwrap_or_default(),
                results: all_results,
                total_results,
                engine: "multi".to_string(),
                search_time_ms: search_time,
            }],
            total_results,
            search_time_ms: search_time,
            duplicates_removed,
        })
    }

    #[tool(name = "list_engines", description = "List all available search engines with their status, latency, and success rate.")]
    fn list_engines(&self) -> Json<EnginesOutput> {
        let engines: Vec<EngineInfo> = self.state.aggregator.engine_names()
            .into_iter()
            .map(|name| {
                let stats = self.state.analytics.engine_stats(name);
                EngineInfo {
                    name: name.to_string(),
                    available: true,
                    avg_latency_ms: stats.0,
                    success_rate: stats.1,
                    total_searches: stats.2,
                }
            })
            .collect();

        Json(EnginesOutput { engines })
    }

    #[tool(name = "health_check", description = "Test all search engines by performing a health check query. Returns status and latency for each engine.")]
    async fn health_check(&self) -> Json<HealthCheckOutput> {
        let state = Arc::clone(&self.state);
        let start = Instant::now();

        let engine_names: Vec<String> = state.aggregator.engine_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let mut engines = Vec::new();

        for engine_name in engine_names {
            let engine_start = Instant::now();
            let result = state.aggregator.search_single(&engine_name, "test", &SearchOptions {
                page_size: 1,
                ..Default::default()
            }).await;

            match result {
                Ok(resp) => engines.push(EngineHealth {
                    name: engine_name,
                    status: "healthy".to_string(),
                    latency_ms: engine_start.elapsed().as_millis() as i64,
                    results_count: resp.results.len() as i64,
                    error: None,
                }),
                Err(e) => engines.push(EngineHealth {
                    name: engine_name,
                    status: "unhealthy".to_string(),
                    latency_ms: engine_start.elapsed().as_millis() as i64,
                    results_count: 0,
                    error: Some(e.to_string()),
                }),
            }
        }

        let total_time = start.elapsed().as_millis() as i64;

        Json(HealthCheckOutput {
            engines,
            total_time_ms: total_time,
        })
    }

    #[tool(name = "get_stats", description = "Get search analytics: total searches, average latency, cache hit rate, and dedup stats.")]
    fn get_stats(&self) -> Json<SearchStatsSnapshot> {
        let stats = self.state.analytics.snapshot();
        Json(stats)
    }

    #[tool(name = "export_results", description = "Search and export results in JSON, CSV, or Markdown format.")]
    async fn export_results(
        &self,
        Parameters(input): Parameters<ExportInput>,
    ) -> Json<ExportOutput> {
        let state = Arc::clone(&self.state);
        let options = SearchOptions {
            page_size: 20,
            ..Default::default()
        };

        let result = state.search(&input.query, input.engine.as_deref(), &options).await;

        match result {
            Ok(response) => {
                let content = match input.format.as_str() {
                    "csv" => export_csv(&response),
                    "markdown" => export_markdown(&response),
                    _ => export_json(&response),
                };
                let result_count = response.results.len() as i64;
                Json(ExportOutput {
                    content,
                    format: input.format,
                    result_count,
                })
            }
            Err(e) => Json(ExportOutput {
                content: format!("Error: {}", e),
                format: input.format,
                result_count: 0,
            }),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

fn parse_time_range(s: &str) -> Option<TimeRange> {
    match s.to_lowercase().as_str() {
        "day" | "24h" | "today" => Some(TimeRange::Day),
        "week" | "7d" => Some(TimeRange::Week),
        "month" | "30d" => Some(TimeRange::Month),
        "year" | "365d" => Some(TimeRange::Year),
        _ => None,
    }
}

fn export_json(response: &mcp_core::error::SearchResponse) -> String {
    serde_json::to_string_pretty(response).unwrap_or_default()
}

fn export_csv(response: &mcp_core::error::SearchResponse) -> String {
    let mut csv = String::from("position,title,url,snippet,engine,published_date\n");
    for r in &response.results {
        csv.push_str(&format!(
            "{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
            r.position.unwrap_or(0),
            r.title.replace('"', "\"\""),
            r.url,
            r.snippet.replace('"', "\"\""),
            r.engine,
            r.published_date.as_deref().unwrap_or(""),
        ));
    }
    csv
}

fn export_markdown(response: &mcp_core::error::SearchResponse) -> String {
    let mut md = format!("## Search Results: \"{}\" ({})\n\n", response.query, response.engine);
    for r in &response.results {
        md.push_str(&format!(
            "{}. **[{}]({})**\n   {}\n\n",
            r.position.unwrap_or(0),
            r.title,
            r.url,
            r.snippet,
        ));
    }
    md
}

// ═══════════════════════════════════════════════════════════════════
// Main
// ═══════════════════════════════════════════════════════════════════

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Starting MCP Web Search Server v{}", env!("CARGO_PKG_VERSION"));

    let args: Vec<String> = std::env::args().collect();

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

    let plugin_dir = args.iter()
        .position(|a| a == "--plugins")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("MCP_WEB_SEARCH_PLUGINS").ok().map(PathBuf::from)
        })
        .or_else(|| {
            let dir = PathBuf::from("plugins");
            if dir.exists() { Some(dir) } else { None }
        });

    let state = Arc::new(AppState::with_config(Some(proxy_config), plugin_dir.as_ref()));
    let server = McpServer { state };

    if http_mode {
        let sse_config = SseTransportConfig {
            host: http_host,
            port: http_port,
        };
        info!("Running in HTTP/SSE mode on {}:{}", sse_config.host, sse_config.port);
        mcp_transport::run_sse_server(sse_config).await?;
    } else {
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
