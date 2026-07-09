use async_trait::async_trait;
use mcp_core::error::{SearchError, SearchOptions, SearchResponse, SearchResult};
use mcp_core::traits::SearchEngine;
use reqwest::Client;
use serde::Deserialize;
use std::time::Instant;
use tracing::debug;

const DEFAULT_SEARXNG_URL: &str = "https://search.sapti.me";

pub struct SearXNGSearch {
    client: Client,
    instance_url: String,
}

#[derive(Deserialize)]
struct SearXNGResponse {
    results: Vec<SearXNGResult>,
    #[serde(default)]
    number_of_results: Option<u64>,
}

#[derive(Deserialize)]
struct SearXNGResult {
    title: String,
    url: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    engine: Option<String>,
    #[serde(default, rename = "publishedDate")]
    published_date: Option<String>,
}

impl SearXNGSearch {
    pub fn new(client: Client) -> Self {
        let instance_url = std::env::var("SEARXNG_URL")
            .unwrap_or_else(|_| DEFAULT_SEARXNG_URL.to_string());
        Self { client, instance_url }
    }

    pub fn with_instance(client: Client, instance_url: &str) -> Self {
        Self {
            client,
            instance_url: instance_url.to_string(),
        }
    }
}

#[async_trait]
impl SearchEngine for SearXNGSearch {
    fn name(&self) -> &str {
        "searxng"
    }

    fn base_url(&self) -> &str {
        &self.instance_url
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
        let page = options.page.unwrap_or(0) + 1;

        let mut url = format!(
            "{}/search?q={}&format=json&pageno={}",
            self.instance_url,
            urlencoding::encode(query),
            page
        );

        if let Some(ref lang) = options.language {
            url.push_str(&format!("&language={}", lang));
        }

        if let Some(time_range) = &options.time_range {
            url.push_str(&format!("&time_range={}", time_range));
        }

        if let Some(safe) = options.safe_search {
            url.push_str(&format!("&safesearch={}", if safe { 2 } else { 0 }));
        }

        debug!("SearXNG search: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let status = response.status();
        if status == 429 {
            return Err(SearchError::RateLimited {
                engine: "searxng".to_string(),
            });
        }
        if status == 403 || status == 503 {
            return Err(SearchError::Blocked {
                engine: "searxng".to_string(),
            });
        }
        if !status.is_success() {
            return Err(SearchError::EngineUnavailable {
                engine: "searxng".to_string(),
                reason: format!("HTTP {}", status),
            });
        }

        let body: SearXNGResponse = response
            .json()
            .await
            .map_err(|e| SearchError::Parse(e.to_string()))?;

        let results: Vec<SearchResult> = body
            .results
            .into_iter()
            .enumerate()
            .map(|(i, r)| SearchResult {
                title: r.title,
                url: r.url,
                snippet: r.content.unwrap_or_default(),
                engine: r.engine.unwrap_or_else(|| "searxng".to_string()),
                published_date: r.published_date,
                thumbnail: None,
                position: Some(i + 1),
            })
            .collect();

        let search_time = start.elapsed().as_millis() as u64;

        Ok(SearchResponse {
            query: query.to_string(),
            total_results: body.number_of_results.map(|n| n as usize).or(Some(results.len())),
            results,
            engine: "searxng".to_string(),
            search_time_ms: search_time,
            next_page_token: None,
        })
    }
}
