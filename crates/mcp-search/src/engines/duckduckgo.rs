use async_trait::async_trait;
use mcp_core::error::{SearchError, SearchOptions, SearchResponse, SearchResult};
use mcp_core::traits::SearchEngine;
use mcp_core::types::SelectorConfig;
use mcp_parser::HtmlSearchParser;
use reqwest::Client;
use std::collections::HashMap;
use std::time::Instant;
use tracing::debug;

pub struct DuckDuckGoSearch {
    client: Client,
    parser: HtmlSearchParser,
}

impl DuckDuckGoSearch {
    pub fn new(client: Client) -> Self {
        let mut parser = HtmlSearchParser::new();
        let config = SelectorConfig {
            search_url: "https://html.duckduckgo.com/html/?q={query}".to_string(),
            result_container: "div.result".to_string(),
            title_selector: "a.result__a".to_string(),
            url_selector: "a.result__a".to_string(),
            snippet_selector: "a.result__snippet".to_string(),
            date_selector: None,
            thumbnail_selector: None,
            pagination_selector: None,
                next_page_selector: Some("input[value='Next']".to_string()),
        };
        let _ = parser.register_engine("duckduckgo", &config);
        Self { client, parser }
    }
}

#[async_trait]
impl SearchEngine for DuckDuckGoSearch {
    fn name(&self) -> &str {
        "duckduckgo"
    }

    fn base_url(&self) -> &str {
        "https://html.duckduckgo.com"
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

        debug!("DuckDuckGo search: {}", query);

        let mut params = HashMap::new();
        params.insert("q", query);

        if let Some(ref lang) = options.language {
            params.insert("kl", lang);
        }

        let response = self
            .client
            .post("https://html.duckduckgo.com/html/")
            .header("User-Agent", crate::user_agents::random_desktop())
            .header("Accept", "text/html")
            .form(&params)
            .send()
            .await
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let status = response.status();
        if status == 429 {
            return Err(SearchError::RateLimited {
                engine: "duckduckgo".to_string(),
            });
        }
        if !status.is_success() {
            return Err(SearchError::EngineUnavailable {
                engine: "duckduckgo".to_string(),
                reason: format!("HTTP {}", status),
            });
        }

        let html = response
            .text()
            .await
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let parsed = self
            .parser
            .parse("duckduckgo", &html, self.base_url())
            .map_err(|e| SearchError::Parse(e.to_string()))?;

        let results: Vec<SearchResult> = parsed
            .into_iter()
            .enumerate()
            .map(|(i, r)| SearchResult {
                title: r.title,
                url: r.url,
                snippet: r.snippet,
                engine: "duckduckgo".to_string(),
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
            engine: "duckduckgo".to_string(),
            search_time_ms: search_time,
            next_page_token: self.parser.extract_next_page("duckduckgo", &html),
        })
    }
}
