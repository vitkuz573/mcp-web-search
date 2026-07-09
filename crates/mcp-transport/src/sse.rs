use axum::{
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

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

/// Server-Sent Events (SSE) session manager
pub struct SseSessionManager {
    sessions: Arc<Mutex<Vec<SseSession>>>,
}

struct SseSession {
    id: String,
    #[allow(dead_code)]
    created_at: std::time::Instant,
}

impl SseSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn create_session(&self) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let session = SseSession {
            id: id.clone(),
            created_at: std::time::Instant::now(),
        };
        self.sessions.lock().await.push(session);
        id
    }

    pub async fn remove_session(&self, id: &str) {
        self.sessions.lock().await.retain(|s| s.id != id);
    }

    pub async fn session_count(&self) -> usize {
        self.sessions.lock().await.len()
    }
}

/// Application state shared across handlers
pub struct SseAppState {
    pub sessions: SseSessionManager,
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "mcp-web-search"
    }))
}

/// Server info endpoint
async fn server_info() -> impl IntoResponse {
    Json(serde_json::json!({
        "name": "mcp-web-search",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "MCP",
        "transport": ["sse", "stdio", "http"],
        "endpoints": {
            "message": "/message",
            "health": "/health",
            "info": "/info",
            "sessions": "/sessions"
        }
    }))
}

/// Message endpoint - clients send MCP messages here via HTTP POST
async fn message_handler(
    State(_app_state): State<Arc<SseAppState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    info!("Received MCP message: {}", body);

    // Process the MCP message and return response
    // This is a simplified HTTP transport - full MCP implementation would route properly
    Json(serde_json::json!({
        "jsonrpc": "2.0",
        "id": body.get("id"),
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {
                "name": "mcp-web-search",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    }))
}

/// List all active sessions
async fn sessions_handler(
    State(app_state): State<Arc<SseAppState>>,
) -> impl IntoResponse {
    let count = app_state.sessions.session_count().await;
    Json(serde_json::json!({
        "active_sessions": count
    }))
}

pub fn create_sse_router(_config: &SseTransportConfig) -> Router {
    let sse_state = Arc::new(SseAppState {
        sessions: SseSessionManager::new(),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_check))
        .route("/info", get(server_info))
        .route("/message", post(message_handler))
        .route("/sessions", get(sessions_handler))
        .with_state(sse_state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

/// Start the HTTP/SSE server
pub async fn run_sse_server(
    config: SseTransportConfig,
) -> anyhow::Result<()> {
    let router = create_sse_router(&config);
    let addr = format!("{}:{}", config.host, config.port);

    info!("Starting HTTP/SSE server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

use axum::extract::State;
