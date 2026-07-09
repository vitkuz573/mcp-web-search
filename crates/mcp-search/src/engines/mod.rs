use mcp_core::traits::SearchEngine;
use mcp_core::error::SearchError;
use std::path::Path;

pub mod google;
pub mod duckduckgo;
pub mod bing;
pub mod brave;
pub mod youtube;
pub mod yahoo;
pub mod custom;

pub use google::GoogleSearch;
pub use duckduckgo::DuckDuckGoSearch;
pub use bing::BingSearch;
pub use brave::BraveSearch;
pub use youtube::YouTubeSearch;
pub use yahoo::YahooSearch;
pub use custom::{CustomSearchEngine, CustomEngineConfig};

pub fn create_engine(
    name: &str,
    client: reqwest::Client,
) -> Option<Box<dyn SearchEngine>> {
    match name.to_lowercase().as_str() {
        "google" => Some(Box::new(GoogleSearch::new(client))),
        "duckduckgo" | "ddg" => Some(Box::new(DuckDuckGoSearch::new(client))),
        "bing" => Some(Box::new(BingSearch::new(client))),
        "brave" => Some(Box::new(BraveSearch::new(client))),
        "youtube" | "yt" => Some(Box::new(YouTubeSearch::new(client))),
        "yahoo" => Some(Box::new(YahooSearch::new(client))),
        _ => None,
    }
}

/// Load custom engines from a TOML config file or a directory of TOML files
pub fn load_custom_engines(
    path: &Path,
    client: reqwest::Client,
) -> Vec<Box<dyn SearchEngine>> {
    let mut engines: Vec<Box<dyn SearchEngine>> = Vec::new();

    if path.is_dir() {
        for engine in CustomSearchEngine::from_directory(path, client) {
            engines.push(Box::new(engine));
        }
    } else if path.is_file() {
        match CustomSearchEngine::from_file(path, client) {
            Ok(engine) => engines.push(Box::new(engine)),
            Err(e) => {
                tracing::warn!("Failed to load custom engine from '{}': {}", path.display(), e);
            }
        }
    }

    engines
}

/// Parse a TOML string into a CustomEngineConfig and create the engine
pub fn create_custom_engine_from_toml(
    toml_str: &str,
    client: reqwest::Client,
) -> Result<Box<dyn SearchEngine>, SearchError> {
    let config: CustomEngineConfig = toml::from_str(toml_str)
        .map_err(|e| SearchError::Config(format!("Failed to parse custom engine TOML: {}", e)))?;
    let engine = CustomSearchEngine::new(config, client)?;
    Ok(Box::new(engine))
}
