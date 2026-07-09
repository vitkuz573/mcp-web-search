use mcp_core::traits::SearchEngine;

pub mod google;
pub mod duckduckgo;
pub mod bing;
pub mod brave;
pub mod youtube;
pub mod yahoo;

pub use google::GoogleSearch;
pub use duckduckgo::DuckDuckGoSearch;
pub use bing::BingSearch;
pub use brave::BraveSearch;
pub use youtube::YouTubeSearch;
pub use yahoo::YahooSearch;

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
