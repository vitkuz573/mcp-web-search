use rmcp::{
    tool_router, tool,
    handler::server::{wrapper::{Parameters, Json}, tool::ToolRouter},
    schemars,
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;

use crate::state::AppState;

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SearchInput {
    pub query: String,
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    #[serde(default)]
    pub page: Option<usize>,
}

fn default_page_size() -> usize {
    10
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct SearchOutput {
    pub query: String,
    pub results: Vec<SearchResultOutput>,
    pub total_results: usize,
    pub engine: String,
    pub search_time_ms: u64,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct SearchResultOutput {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub engine: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct MultiSearchInput {
    pub queries: Vec<MultiSearchQuery>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct MultiSearchQuery {
    pub engine: String,
    pub query: String,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct MultiSearchOutput {
    pub responses: Vec<SearchOutput>,
    pub total_results: usize,
    pub search_time_ms: u64,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct EnginesInput {}

#[derive(Serialize, schemars::JsonSchema)]
pub struct EnginesOutput {
    pub engines: Vec<EngineInfo>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct EngineInfo {
    pub name: String,
    pub available: bool,
}

pub struct McpTools {
    pub state: Arc<AppState>,
}

impl McpTools {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub fn tool_router(&self) -> ToolRouter<Self> {
        ToolRouter::new()
    }
}
