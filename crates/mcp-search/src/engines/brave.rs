use async_trait::async_trait;
use mcp_core::error::{SearchError, SearchOptions, SearchResponse, SearchResult};
use mcp_core::traits::SearchEngine;
use mcp_core::types::SelectorConfig;
use mcp_parser::HtmlSearchParser;
use reqwest::Client;
use std::time::Instant;
use tracing::debug;

pub struct BraveSearch {
    client: Client,
    parser: HtmlSearchParser,
}

impl BraveSearch {
    pub fn new(client: Client) -> Self {
        let mut parser = HtmlSearchParser::new();
        let config = SelectorConfig {
            search_url: "https://search.brave.com/search?q={query}".to_string(),
            result_container: "div.snippet[data-type=\"web\"]".to_string(),
            title_selector: "div.title".to_string(),
            url_selector: "a".to_string(),
            snippet_selector: ".generic-snippet .content".to_string(),
            date_selector: None,
            thumbnail_selector: None,
            pagination_selector: None,
            next_page_selector: Some("a.next".to_string()),
        };
        let _ = parser.register_engine("brave", &config);
        Self { client, parser }
    }
}

#[async_trait]
impl SearchEngine for BraveSearch {
    fn name(&self) -> &str {
        "brave"
    }

    fn base_url(&self) -> &str {
        "https://search.brave.com"
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn search(
        &self,
        query: &str,
        _options: &SearchOptions,
    ) -> Result<SearchResponse, SearchError> {
        let start = Instant::now();
        let url = format!(
            "https://search.brave.com/search?q={}",
            urlencoding::encode(query)
        );

        debug!("Brave search: {}", url);

        let response = self
            .client
            .get(&url)
            .header("User-Agent", crate::user_agents::random_desktop())
            .header("Accept", "text/html,application/xhtml+xml")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let status = response.status();
        if status == 429 {
            return Err(SearchError::RateLimited {
                engine: "brave".to_string(),
            });
        }
        if !status.is_success() {
            return Err(SearchError::EngineUnavailable {
                engine: "brave".to_string(),
                reason: format!("HTTP {}", status),
            });
        }

        let html = response
            .text()
            .await
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let parsed = self
            .parser
            .parse("brave", &html, self.base_url())
            .map_err(|e| SearchError::Parse(e.to_string()))?;

        let results: Vec<SearchResult> = parsed
            .into_iter()
            .enumerate()
            .map(|(i, r)| SearchResult {
                title: r.title,
                url: r.url,
                snippet: r.snippet,
                engine: "brave".to_string(),
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
            engine: "brave".to_string(),
            search_time_ms: search_time,
            next_page_token: self.parser.extract_next_page("brave", &html),
        })
    }
}
