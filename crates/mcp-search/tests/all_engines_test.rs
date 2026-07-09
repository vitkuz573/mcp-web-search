#[cfg(test)]
mod tests {
    use mcp_core::error::SearchOptions;
    use mcp_core::traits::SearchEngine;
    use reqwest::Client;

    fn create_client() -> Client {
        Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("Failed to create client")
    }

    #[tokio::test]
    async fn test_google_search() {
        let client = create_client();
        let engine = mcp_search::engines::GoogleSearch::new(client);
        let options = SearchOptions {
            language: Some("en".to_string()),
            page_size: 10,
            ..Default::default()
        };

        let result = engine.search("rust programming language", &options).await;
        
        match result {
            Ok(response) => {
                println!("Google search successful!");
                println!("Results: {}", response.results.len());
                for (i, r) in response.results.iter().enumerate() {
                    println!("{}. {} - {}", i + 1, r.title, r.url);
                }
                assert!(!response.results.is_empty());
            }
            Err(e) => {
                println!("Google search error (expected due to CAPTCHA): {}", e);
                // Google might block automated requests, so we just log it
            }
        }
    }

    #[tokio::test]
    async fn test_duckduckgo_search() {
        let client = create_client();
        let engine = mcp_search::engines::DuckDuckGoSearch::new(client);
        let options = SearchOptions {
            language: Some("en".to_string()),
            page_size: 10,
            ..Default::default()
        };

        let result = engine.search("rust programming language", &options).await;
        
        match result {
            Ok(response) => {
                println!("DuckDuckGo search successful!");
                println!("Results: {}", response.results.len());
                for (i, r) in response.results.iter().enumerate() {
                    println!("{}. {} - {}", i + 1, r.title, r.url);
                }
                assert!(!response.results.is_empty());
            }
            Err(e) => {
                println!("DuckDuckGo search error (expected due to CAPTCHA): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_bing_search() {
        let client = create_client();
        let engine = mcp_search::engines::BingSearch::new(client);
        let options = SearchOptions {
            language: Some("en".to_string()),
            page_size: 10,
            ..Default::default()
        };

        let result = engine.search("rust programming language", &options).await;
        
        match result {
            Ok(response) => {
                println!("Bing search successful!");
                println!("Results: {}", response.results.len());
                for (i, r) in response.results.iter().enumerate() {
                    println!("{}. {} - {}", i + 1, r.title, r.url);
                }
                assert!(!response.results.is_empty());
            }
            Err(e) => {
                panic!("Bing search failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_brave_search() {
        let client = create_client();
        let engine = mcp_search::engines::BraveSearch::new(client);
        let options = SearchOptions {
            language: Some("en".to_string()),
            page_size: 10,
            ..Default::default()
        };

        let result = engine.search("rust programming language", &options).await;
        
        match result {
            Ok(response) => {
                println!("Brave search successful!");
                println!("Results: {}", response.results.len());
                for (i, r) in response.results.iter().enumerate() {
                    println!("{}. {} - {}", i + 1, r.title, r.url);
                }
                assert!(!response.results.is_empty());
            }
            Err(e) => {
                println!("Brave search error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_youtube_search() {
        let client = create_client();
        let engine = mcp_search::engines::YouTubeSearch::new(client);
        let options = SearchOptions {
            language: Some("en".to_string()),
            page_size: 10,
            ..Default::default()
        };

        let result = engine.search("rust tutorial", &options).await;
        
        match result {
            Ok(response) => {
                println!("YouTube search successful!");
                println!("Results: {}", response.results.len());
                for (i, r) in response.results.iter().enumerate() {
                    println!("{}. {} - {}", i + 1, r.title, r.url);
                }
                assert!(!response.results.is_empty());
            }
            Err(e) => {
                println!("YouTube search error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_yahoo_search() {
        let client = create_client();
        let engine = mcp_search::engines::YahooSearch::new(client);
        let options = SearchOptions {
            language: Some("en".to_string()),
            page_size: 10,
            ..Default::default()
        };

        let result = engine.search("rust programming language", &options).await;
        
        match result {
            Ok(response) => {
                println!("Yahoo search successful!");
                println!("Results: {}", response.results.len());
                for (i, r) in response.results.iter().enumerate() {
                    println!("{}. {} - {}", i + 1, r.title, r.url);
                }
                assert!(!response.results.is_empty());
            }
            Err(e) => {
                println!("Yahoo search error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_aggregator_search_all() {
        let client = create_client();
        let mut aggregator = mcp_search::SearchAggregator::new();
        
        let engines: Vec<Box<dyn mcp_core::traits::SearchEngine>> = vec![
            Box::new(mcp_search::engines::BingSearch::new(client.clone())),
            Box::new(mcp_search::engines::BraveSearch::new(client.clone())),
        ];
        
        for engine in engines {
            aggregator.add_engine(std::sync::Arc::from(engine));
        }

        let options = SearchOptions {
            language: Some("en".to_string()),
            page_size: 5,
            ..Default::default()
        };

        let result = aggregator.search_all("rust programming", &options).await;
        
        println!("Aggregator search completed!");
        println!("Total responses: {}", result.responses.len());
        println!("Total results: {}", result.total_results);
        println!("Search time: {}ms", result.search_time_ms);
        
        for response in &result.responses {
            println!("\n{}: {} results", response.engine, response.results.len());
        }
    }
}
