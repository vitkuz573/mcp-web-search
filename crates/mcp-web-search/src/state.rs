use mcp_core::error::{SearchError, SearchOptions, SearchResponse};
use mcp_search::SearchAggregator;
use moka::future::Cache;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

pub struct AppState {
    pub aggregator: SearchAggregator,
    pub cache: Cache<String, SearchResponse>,
}

impl AppState {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        let mut aggregator = SearchAggregator::new();

        let engine_names = vec!["google", "duckduckgo", "bing", "brave", "youtube", "yahoo"];
        for name in engine_names {
            if let Some(engine) = mcp_search::create_engine(name, client.clone()) {
                info!("Registered search engine: {}", name);
                let engine: Arc<dyn mcp_core::traits::SearchEngine> = Arc::from(engine);
                aggregator.add_engine(engine);
            }
        }

        let cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(3600))
            .build();

        Self { aggregator, cache }
    }

    pub async fn search(
        &self,
        query: &str,
        engine: Option<&str>,
        options: &SearchOptions,
    ) -> Result<SearchResponse, SearchError> {
        let cache_key = format!(
            "{}:{}:{}",
            query,
            engine.unwrap_or("multi"),
            serde_json::to_string(options).unwrap_or_default()
        );

        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(cached);
        }

        let result = if let Some(engine_name) = engine {
            self.aggregator.search_single(engine_name, query, options).await?
        } else {
            let multi = self.aggregator.search_all(query, options).await;
            if multi.responses.is_empty() {
                return Err(SearchError::NoResults {
                    query: query.to_string(),
                });
            }
            multi.responses.into_iter().next().unwrap()
        };

        self.cache.insert(cache_key, result.clone()).await;
        Ok(result)
    }
}
