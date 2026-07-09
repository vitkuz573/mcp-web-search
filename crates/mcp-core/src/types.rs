use serde::{Deserialize, Serialize};

pub use crate::error::{SearchError, SearchOptions, SearchResponse, SearchResult, TimeRange};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub search_engine: String,
    pub base_url: String,
    pub selectors_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorConfig {
    pub search_url: String,
    pub result_container: String,
    pub title_selector: String,
    pub url_selector: String,
    pub snippet_selector: String,
    pub date_selector: Option<String>,
    pub thumbnail_selector: Option<String>,
    pub pagination_selector: Option<String>,
    pub next_page_selector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_requests_per_minute: u32,
    pub max_requests_per_hour: u32,
    pub backoff_multiplier: f64,
    pub max_backoff_ms: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_per_minute: 30,
            max_requests_per_hour: 500,
            backoff_multiplier: 2.0,
            max_backoff_ms: 60_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_seconds: u64,
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_seconds: 3600,
            max_entries: 10_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub rate_limit: RateLimitConfig,
    pub cache: CacheConfig,
    pub engines: Vec<super::error::EngineConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            log_level: "info".to_string(),
            rate_limit: RateLimitConfig::default(),
            cache: CacheConfig::default(),
            engines: Vec::new(),
        }
    }
}
