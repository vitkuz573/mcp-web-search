use mcp_core::error::{SearchError, SearchOptions, SearchResponse, MultiSearchResponse};
use mcp_core::traits::SearchEngine;
use futures::future::join_all;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, warn};

pub struct SearchAggregator {
    engines: Vec<Arc<dyn SearchEngine>>,
}

impl SearchAggregator {
    pub fn new() -> Self {
        Self {
            engines: Vec::new(),
        }
    }

    pub fn add_engine(&mut self, engine: Arc<dyn SearchEngine>) {
        self.engines.push(engine);
    }

    pub fn engine_names(&self) -> Vec<&str> {
        self.engines.iter().map(|e| e.name()).collect()
    }

    pub async fn search_single(
        &self,
        engine_name: &str,
        query: &str,
        options: &SearchOptions,
    ) -> Result<SearchResponse, SearchError> {
        let engine = self.engines
            .iter()
            .find(|e| e.name() == engine_name)
            .ok_or_else(|| SearchError::EngineUnavailable {
                engine: engine_name.to_string(),
                reason: "Engine not found".to_string(),
            })?;

        engine.search(query, options).await
    }

    pub async fn search_multi(
        &self,
        queries: &[(String, String)],
        options: &SearchOptions,
    ) -> Vec<Result<SearchResponse, SearchError>> {
        let futures: Vec<_> = queries
            .iter()
            .map(|(engine, query)| {
                let engine = engine.clone();
                let query = query.clone();
                let options = options.clone();
                async move { self.search_single(&engine, &query, &options).await }
            })
            .collect();

        join_all(futures).await
    }

    pub async fn search_all(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> MultiSearchResponse {
        let start = Instant::now();

        let futures: Vec<_> = self.engines
            .iter()
            .map(|engine| {
                let engine = Arc::clone(engine);
                let query = query.to_string();
                let options = options.clone();
                async move {
                    let result = engine.search(&query, &options).await;
                    debug!("Engine {} completed: {:?}", engine.name(), result.is_ok());
                    result
                }
            })
            .collect();

        let results = join_all(futures).await;

        let responses: Vec<SearchResponse> = results
            .into_iter()
            .filter_map(|r| match r {
                Ok(resp) => Some(resp),
                Err(e) => {
                    warn!("Search engine error: {}", e);
                    None
                }
            })
            .collect();

        let total_results: usize = responses.iter().map(|r| r.results.len()).sum();
        let search_time = start.elapsed().as_millis() as u64;

        MultiSearchResponse {
            query: query.to_string(),
            responses,
            total_results,
            search_time_ms: search_time,
        }
    }
}
