use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Rate limited by {engine}")]
    RateLimited { engine: String },

    #[error("Engine unavailable: {engine} — {reason}")]
    EngineUnavailable { engine: String, reason: String },

    #[error("No results found for query: {query}")]
    NoResults { query: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Blocked by captcha or bot detection on {engine}")]
    Blocked { engine: String },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Serialize for SearchError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SearchError", 2)?;
        state.serialize_field("type", &self.error_type())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

impl SearchError {
    pub fn error_type(&self) -> &'static str {
        match self {
            Self::Network(_) => "network",
            Self::Parse(_) => "parse",
            Self::RateLimited { .. } => "rate_limited",
            Self::EngineUnavailable { .. } => "engine_unavailable",
            Self::NoResults { .. } => "no_results",
            Self::Config(_) => "config",
            Self::Timeout(_) => "timeout",
            Self::Blocked { .. } => "blocked",
            Self::Other(_) => "other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub engine: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total_results: Option<usize>,
    pub engine: String,
    pub search_time_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSearchResponse {
    pub query: String,
    pub responses: Vec<SearchResponse>,
    pub total_results: usize,
    pub search_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe_search: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<TimeRange>,
}

fn default_page_size() -> usize {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeRange {
    Day,
    Week,
    Month,
    Year,
}

impl fmt::Display for TimeRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Day => write!(f, "day"),
            Self::Week => write!(f, "week"),
            Self::Month => write!(f, "month"),
            Self::Year => write!(f, "year"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub name: String,
    pub enabled: bool,
    pub base_url: String,
    #[serde(default)]
    pub weight: f64,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
}

fn default_timeout() -> u64 {
    5_000
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            enabled: true,
            base_url: String::new(),
            weight: 1.0,
            timeout_ms: default_timeout(),
            headers: std::collections::HashMap::new(),
        }
    }
}
