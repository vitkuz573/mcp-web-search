use async_trait::async_trait;
use mcp_core::error::{SearchError, SearchOptions, SearchResponse, SearchResult};
use mcp_core::traits::SearchEngine;
use mcp_core::types::SelectorConfig;
use mcp_parser::HtmlSearchParser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Configuration for a custom search engine plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEngineConfig {
    /// Engine name (used as identifier)
    pub name: String,

    /// Human-readable description
    #[serde(default)]
    pub description: String,

    /// Base URL of the search engine (e.g., "https://example.com")
    pub base_url: String,

    /// URL template for search queries. Use {query}, {lang}, {region}, {page}, {page_size} placeholders
    pub search_url_template: String,

    /// CSS selector for the container element that holds each search result
    pub result_container: String,

    /// CSS selector for the title element within a result container
    pub title_selector: String,

    /// CSS selector for the URL element within a result container
    pub url_selector: String,

    /// CSS selector for the snippet element within a result container
    pub snippet_selector: String,

    /// CSS selector for the date element (optional)
    #[serde(default)]
    pub date_selector: Option<String>,

    /// CSS selector for the thumbnail element (optional)
    #[serde(default)]
    pub thumbnail_selector: Option<String>,

    /// CSS selector for the next page link (optional)
    #[serde(default)]
    pub next_page_selector: Option<String>,

    /// Custom HTTP headers to send with requests
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Custom User-Agent string (optional, random desktop UA used if not set)
    #[serde(default)]
    pub user_agent: Option<String>,

    /// Request timeout in milliseconds (default: 10000)
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    /// URL attribute to extract href from (default: "href")
    #[serde(default = "default_url_attr")]
    pub url_attr: String,

    /// Whether this engine is enabled (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Weight for result ordering in multi-engine search (default: 1.0)
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_timeout_ms() -> u64 {
    10_000
}
fn default_url_attr() -> String {
    "href".to_string()
}
fn default_true() -> bool {
    true
}
fn default_weight() -> f64 {
    1.0
}

/// A custom search engine loaded from TOML configuration
pub struct CustomSearchEngine {
    config: CustomEngineConfig,
    client: Client,
    parser: HtmlSearchParser,
}

impl CustomSearchEngine {
    pub fn new(config: CustomEngineConfig, client: Client) -> Result<Self, SearchError> {
        let mut parser = HtmlSearchParser::new();

        let selector_config = SelectorConfig {
            search_url: config.search_url_template.clone(),
            result_container: config.result_container.clone(),
            title_selector: config.title_selector.clone(),
            url_selector: config.url_selector.clone(),
            snippet_selector: config.snippet_selector.clone(),
            date_selector: config.date_selector.clone(),
            thumbnail_selector: config.thumbnail_selector.clone(),
            pagination_selector: None,
            next_page_selector: config.next_page_selector.clone(),
        };

        parser
            .register_engine(&config.name, &selector_config)
            .map_err(|e| SearchError::Config(format!("Invalid CSS selector for engine '{}': {}", config.name, e)))?;

        info!("Loaded custom engine: {} ({})", config.name, config.description);

        Ok(Self {
            config,
            client,
            parser,
        })
    }

    /// Load a custom engine from a TOML file
    pub fn from_file(path: &Path, client: Client) -> Result<Self, SearchError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SearchError::Config(format!("Failed to read plugin file '{}': {}", path.display(), e)))?;

        let config: CustomEngineConfig = toml::from_str(&content)
            .map_err(|e| SearchError::Config(format!("Failed to parse plugin file '{}': {}", path.display(), e)))?;

        Self::new(config, client)
    }

    /// Load all custom engines from a directory
    pub fn from_directory(dir: &Path, client: Client) -> Vec<Self> {
        let mut engines = Vec::new();

        if !dir.exists() {
            warn!("Plugin directory does not exist: {}", dir.display());
            return engines;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read plugin directory '{}': {}", dir.display(), e);
                return engines;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                match Self::from_file(&path, client.clone()) {
                    Ok(engine) => {
                        if engine.config.enabled {
                            engines.push(engine);
                        } else {
                            info!("Skipping disabled custom engine: {}", path.display());
                        }
                    }
                    Err(e) => {
                        warn!("Failed to load custom engine from '{}': {}", path.display(), e);
                    }
                }
            }
        }

        engines
    }

    fn build_url(&self, query: &str, options: &SearchOptions) -> String {
        let lang = options.language.as_deref().unwrap_or("en");
        let region = options.region.as_deref().unwrap_or("us");
        let page = options.page.unwrap_or(0);
        let page_size = options.page_size;

        let url = self
            .config
            .search_url_template
            .replace("{query}", &urlencoding::encode(query))
            .replace("{lang}", lang)
            .replace("{region}", region)
            .replace("{page}", &page.to_string())
            .replace("{page_size}", &page_size.to_string())
            .replace("{first}", &(page * page_size + 1).to_string());

        url
    }
}

#[async_trait]
impl SearchEngine for CustomSearchEngine {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn base_url(&self) -> &str {
        &self.config.base_url
    }

    fn is_available(&self) -> bool {
        self.config.enabled
    }

    async fn search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<SearchResponse, SearchError> {
        let start = Instant::now();
        let url = self.build_url(query, options);

        debug!("Custom engine '{}' search: {}", self.config.name, url);

        let mut request = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_millis(self.config.timeout_ms));

        // Add custom headers
        for (key, value) in &self.config.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        // Use custom user agent or random desktop UA
        let default_ua = crate::user_agents::random_desktop();
        let user_agent = self
            .config
            .user_agent
            .as_deref()
            .unwrap_or(&default_ua);
        request = request.header("User-Agent", user_agent);
        request = request.header("Accept", "text/html,application/xhtml+xml");
        request = request.header("Accept-Language", "en-US,en;q=0.9");

        let response = request
            .send()
            .await
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let status = response.status();
        if status == 429 {
            return Err(SearchError::RateLimited {
                engine: self.config.name.clone(),
            });
        }
        if status == 403 || status == 503 {
            return Err(SearchError::Blocked {
                engine: self.config.name.clone(),
            });
        }
        if !status.is_success() {
            return Err(SearchError::EngineUnavailable {
                engine: self.config.name.clone(),
                reason: format!("HTTP {}", status),
            });
        }

        let html = response
            .text()
            .await
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let parsed = self
            .parser
            .parse(&self.config.name, &html, &self.config.base_url)
            .map_err(|e| SearchError::Parse(e.to_string()))?;

        let results: Vec<SearchResult> = parsed
            .into_iter()
            .enumerate()
            .map(|(i, r)| SearchResult {
                title: r.title,
                url: r.url,
                snippet: r.snippet,
                engine: self.config.name.clone(),
                published_date: r.published_date,
                thumbnail: r.thumbnail,
                position: Some(i + 1),
            })
            .collect();

        let search_time = start.elapsed().as_millis() as u64;

        Ok(SearchResponse {
            query: query.to_string(),
            total_results: Some(results.len()),
            results,
            engine: self.config.name.clone(),
            search_time_ms: search_time,
            next_page_token: self.parser.extract_next_page(&self.config.name, &html),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plugin_config() {
        let toml_str = r#"
name = "example"
description = "Example search engine"
base_url = "https://example.com"
search_url_template = "https://example.com/search?q={query}&lang={lang}"
result_container = "div.result"
title_selector = "h3"
url_selector = "a"
snippet_selector = "p"
"#;

        let config: CustomEngineConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.name, "example");
        assert_eq!(config.base_url, "https://example.com");
        assert!(config.enabled);
        assert_eq!(config.timeout_ms, 10000);
    }

    #[test]
    fn test_build_url() {
        let toml_str = r#"
name = "test"
description = "Test"
base_url = "https://test.com"
search_url_template = "https://test.com/search?q={query}&lang={lang}&region={region}&page={page}"
result_container = "div.r"
title_selector = "h3"
url_selector = "a"
snippet_selector = "p"
"#;

        let config: CustomEngineConfig = toml::from_str(toml_str).unwrap();
        let client = Client::new();
        let engine = CustomSearchEngine::new(config, client).unwrap();

        let options = SearchOptions {
            language: Some("de".to_string()),
            region: Some("at".to_string()),
            page: Some(2),
            page_size: 10,
            ..Default::default()
        };

        let url = engine.build_url("rust programming", &options);
        assert!(url.contains("rust%20programming") || url.contains("rust+programming"));
        assert!(url.contains("lang=de"));
        assert!(url.contains("region=at"));
        assert!(url.contains("page=2"));
    }
}
