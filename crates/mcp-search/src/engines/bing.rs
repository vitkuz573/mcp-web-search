use async_trait::async_trait;
use mcp_core::error::{SearchError, SearchOptions, SearchResponse, SearchResult};
use mcp_core::traits::SearchEngine;
use mcp_core::types::SelectorConfig;
use mcp_parser::HtmlSearchParser;
use reqwest::Client;
use std::time::Instant;
use tracing::debug;

pub struct BingSearch {
    client: Client,
    parser: HtmlSearchParser,
}

impl BingSearch {
    pub fn new(client: Client) -> Self {
        let mut parser = HtmlSearchParser::new();
        let config = SelectorConfig {
            search_url: "https://www.bing.com/search?q={query}&setlang={lang}".to_string(),
            result_container: "li.b_algo".to_string(),
            title_selector: "h2 a".to_string(),
            url_selector: "h2 a".to_string(),
            snippet_selector: "div.b_caption p".to_string(),
            date_selector: None,
            thumbnail_selector: None,
            pagination_selector: None,
            next_page_selector: Some("a.sb_pagN".to_string()),
        };
        let _ = parser.register_engine("bing", &config);
        Self { client, parser }
    }

    fn build_url(&self, query: &str, options: &SearchOptions) -> String {
        let lang = options.language.as_deref().unwrap_or("en");
        let page = options.page.unwrap_or(0);
        let first = page * options.page_size + 1;

        format!(
            "https://www.bing.com/search?q={}&setlang={}&mkt={}-{}&first={}",
            urlencoding::encode(query),
            lang,
            lang.to_uppercase(),
            options.region.as_deref().unwrap_or("us").to_uppercase(),
            first
        )
    }
}

#[async_trait]
impl SearchEngine for BingSearch {
    fn name(&self) -> &str {
        "bing"
    }

    fn base_url(&self) -> &str {
        "https://www.bing.com"
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<SearchResponse, SearchError> {
        let start = Instant::now();
        let url = self.build_url(query, options);

        debug!("Bing search: {}", url);

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
                engine: "bing".to_string(),
            });
        }
        if status == 403 || status == 503 {
            return Err(SearchError::Blocked {
                engine: "bing".to_string(),
            });
        }
        if !status.is_success() {
            return Err(SearchError::EngineUnavailable {
                engine: "bing".to_string(),
                reason: format!("HTTP {}", status),
            });
        }

        let html = response
            .text()
            .await
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let parsed = self
            .parser
            .parse("bing", &html, self.base_url())
            .map_err(|e| SearchError::Parse(e.to_string()))?;

        let results: Vec<SearchResult> = parsed
            .into_iter()
            .enumerate()
            .map(|(i, r)| SearchResult {
                title: r.title,
                url: r.url,
                snippet: r.snippet,
                engine: "bing".to_string(),
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
            engine: "bing".to_string(),
            search_time_ms: search_time,
            next_page_token: self.parser.extract_next_page("bing", &html),
        })
    }
}
