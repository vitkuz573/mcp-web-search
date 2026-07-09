use async_trait::async_trait;
use mcp_core::error::{SearchError, SearchOptions, SearchResponse, SearchResult};
use mcp_core::traits::SearchEngine;
use mcp_core::types::SelectorConfig;
use mcp_parser::HtmlSearchParser;
use reqwest::Client;
use std::time::Instant;
use tracing::debug;

pub struct YahooSearch {
    client: Client,
    parser: HtmlSearchParser,
}

impl YahooSearch {
    pub fn new(client: Client) -> Self {
        let mut parser = HtmlSearchParser::new();
        let config = SelectorConfig {
            search_url: "https://search.yahoo.com/search?p={query}".to_string(),
            result_container: "div.algo".to_string(),
            title_selector: ".compTitle h3".to_string(),
            url_selector: ".compTitle a".to_string(),
            snippet_selector: ".compText p".to_string(),
            date_selector: None,
            thumbnail_selector: None,
            pagination_selector: None,
            next_page_selector: Some("a.next".to_string()),
        };
        let _ = parser.register_engine("yahoo", &config);
        Self { client, parser }
    }
}

#[async_trait]
impl SearchEngine for YahooSearch {
    fn name(&self) -> &str {
        "yahoo"
    }

    fn base_url(&self) -> &str {
        "https://search.yahoo.com"
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
            "https://search.yahoo.com/search?p={}",
            urlencoding::encode(query)
        );

        debug!("Yahoo search: {}", url);

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
                engine: "yahoo".to_string(),
            });
        }
        if !status.is_success() {
            return Err(SearchError::EngineUnavailable {
                engine: "yahoo".to_string(),
                reason: format!("HTTP {}", status),
            });
        }

        let html = response
            .text()
            .await
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let parsed = self
            .parser
            .parse("yahoo", &html, self.base_url())
            .map_err(|e| SearchError::Parse(e.to_string()))?;

        let results: Vec<SearchResult> = parsed
            .into_iter()
            .enumerate()
            .map(|(i, r)| SearchResult {
                title: r.title,
                url: r.url,
                snippet: r.snippet,
                engine: "yahoo".to_string(),
                published_date: None,
                thumbnail: None,
                position: Some(i + 1),
            })
            .collect();

        let search_time = start.elapsed().as_millis() as u64;

        Ok(SearchResponse {
            query: query.to_string(),
            total_results: Some(results.len()),
            results,
            engine: "yahoo".to_string(),
            search_time_ms: search_time,
            next_page_token: self.parser.extract_next_page("yahoo", &html),
        })
    }
}
