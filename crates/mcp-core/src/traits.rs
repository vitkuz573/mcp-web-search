use async_trait::async_trait;
use crate::error::{SearchError, SearchOptions, SearchResponse};

#[async_trait]
pub trait SearchEngine: Send + Sync {
    fn name(&self) -> &str;
    fn base_url(&self) -> &str;
    fn is_available(&self) -> bool;

    async fn search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<SearchResponse, SearchError>;

    async fn health_check(&self) -> Result<(), SearchError> {
        Ok(())
    }
}
