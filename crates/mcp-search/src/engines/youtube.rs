use async_trait::async_trait;
use mcp_core::error::{SearchError, SearchOptions, SearchResponse, SearchResult};
use mcp_core::traits::SearchEngine;
use mcp_core::types::SelectorConfig;
use mcp_parser::HtmlSearchParser;
use reqwest::Client;
use std::time::Instant;
use tracing::debug;

pub struct YouTubeSearch {
    client: Client,
    parser: HtmlSearchParser,
}

impl YouTubeSearch {
    pub fn new(client: Client) -> Self {
        let mut parser = HtmlSearchParser::new();
        let config = SelectorConfig {
            search_url: "https://www.youtube.com/results?search_query={query}".to_string(),
            result_container: "div#contents ytd-video-renderer".to_string(),
            title_selector: "a#video-title".to_string(),
            url_selector: "a#video-title".to_string(),
            snippet_selector: "div#description-text".to_string(),
            date_selector: Some("span.ytd-video-meta-block".to_string()),
            thumbnail_selector: Some("img#img".to_string()),
            pagination_selector: None,
            next_page_selector: None,
        };
        let _ = parser.register_engine("youtube", &config);
        Self { client, parser }
    }
}

#[async_trait]
impl SearchEngine for YouTubeSearch {
    fn name(&self) -> &str {
        "youtube"
    }

    fn base_url(&self) -> &str {
        "https://www.youtube.com"
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
            "https://www.youtube.com/results?search_query={}",
            urlencoding::encode(query)
        );

        debug!("YouTube search: {}", url);

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
                engine: "youtube".to_string(),
            });
        }
        if !status.is_success() {
            return Err(SearchError::EngineUnavailable {
                engine: "youtube".to_string(),
                reason: format!("HTTP {}", status),
            });
        }

        let html = response
            .text()
            .await
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let parsed = self
            .parser
            .parse("youtube", &html, self.base_url())
            .map_err(|e| SearchError::Parse(e.to_string()))?;

        let results: Vec<SearchResult> = parsed
            .into_iter()
            .enumerate()
            .map(|(i, r)| SearchResult {
                title: r.title,
                url: if r.url.starts_with("http") {
                    r.url
                } else {
                    format!("https://www.youtube.com{}", r.url)
                },
                snippet: r.snippet,
                engine: "youtube".to_string(),
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
            engine: "youtube".to_string(),
            search_time_ms: search_time,
            next_page_token: None,
        })
    }
}
