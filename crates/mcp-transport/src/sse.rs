use axum::{
    response::IntoResponse,
    routing::get,
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct SseTransportConfig {
    pub host: String,
    pub port: u16,
}

impl Default for SseTransportConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
        }
    }
}

pub fn create_sse_router(_config: &SseTransportConfig) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_check))
        .route("/info", get(server_info))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

async fn health_check() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "status": "ok",
        "service": "mcp-web-search"
    }))
}

async fn server_info() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "name": "mcp-web-search",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "MCP",
        "transport": ["sse", "stdio"],
        "engines": ["google", "duckduckgo", "bing", "brave", "youtube", "yahoo"]
    }))
}
