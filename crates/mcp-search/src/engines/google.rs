use async_trait::async_trait;
use mcp_core::error::{SearchError, SearchOptions, SearchResponse, SearchResult};
use mcp_core::traits::SearchEngine;
use mcp_core::types::SelectorConfig;
use mcp_parser::HtmlSearchParser;
use reqwest::Client;
use std::time::Instant;
use tracing::{debug};

pub struct GoogleSearch {
    client: Client,
    parser: HtmlSearchParser,
}

impl GoogleSearch {
    pub fn new(client: Client) -> Self {
        let mut parser = HtmlSearchParser::new();
        let config = SelectorConfig {
            search_url: "https://www.google.com/search?q={query}&hl={lang}".to_string(),
            result_container: "div.g".to_string(),
            title_selector: "h3".to_string(),
            url_selector: "a".to_string(),
            snippet_selector: "div.VwiC3b, span.aCOpRe".to_string(),
            date_selector: Some("span.LEwnzc, span.MUxGbd".to_string()),
            thumbnail_selector: None,
            pagination_selector: None,
            next_page_selector: Some("a#pnnext".to_string()),
        };
        let _ = parser.register_engine("google", &config);
        Self { client, parser }
    }

    fn build_url(&self, query: &str, options: &SearchOptions) -> String {
        let lang = options.language.as_deref().unwrap_or("en");
        let page = options.page.unwrap_or(0);
        let start = page * options.page_size;

        let mut url = format!(
            "https://www.google.com/search?q={}&hl={}&start={}",
            urlencoding::encode(query),
            lang,
            start
        );

        if let Some(ref region) = options.region {
            url.push_str(&format!("&gl={}", region));
        }

        if let Some(true) = options.safe_search {
            url.push_str("&safe=active");
        }

        url
    }
}

#[async_trait]
impl SearchEngine for GoogleSearch {
    fn name(&self) -> &str {
        "google"
    }

    fn base_url(&self) -> &str {
        "https://www.google.com"
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

        debug!("Google search: {}", url);

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
                engine: "google".to_string(),
            });
        }
        if status == 403 || status == 503 {
            return Err(SearchError::Blocked {
                engine: "google".to_string(),
            });
        }
        if !status.is_success() {
            return Err(SearchError::EngineUnavailable {
                engine: "google".to_string(),
                reason: format!("HTTP {}", status),
            });
        }

        let html = response
            .text()
            .await
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let parsed = self
            .parser
            .parse("google", &html, self.base_url())
            .map_err(|e| SearchError::Parse(e.to_string()))?;

        let results: Vec<SearchResult> = parsed
            .into_iter()
            .enumerate()
            .map(|(i, r)| SearchResult {
                title: r.title,
                url: r.url,
                snippet: r.snippet,
                engine: "google".to_string(),
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
            engine: "google".to_string(),
            search_time_ms: search_time,
            next_page_token: self.parser.extract_next_page("google", &html),
        })
    }
}
