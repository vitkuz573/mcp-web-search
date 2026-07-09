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
    async fn test_bing_with_locale() {
        let client = create_client();
        let engine = mcp_search::engines::BingSearch::new(client);
        let options = SearchOptions {
            language: Some("en".to_string()),
            region: Some("US".to_string()),
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
    async fn test_bing_detects_blocked() {
        let client = create_client();
        let engine = mcp_search::engines::BingSearch::new(client);
        let options = SearchOptions::default();

        // This query might trigger CAPTCHA on some IPs
        let result = engine.search("test query 12345", &options).await;
        
        match result {
            Ok(response) => {
                println!("Bing responded with {} results", response.results.len());
            }
            Err(e) => {
                println!("Bing error: {}", e);
                // Don't panic - it's expected that some queries get blocked
            }
        }
    }

    #[tokio::test]
    async fn test_error_handling() {
        let client = create_client();
        let engine = mcp_search::engines::BingSearch::new(client);
        let options = SearchOptions::default();

        let result = engine.search("rust programming", &options).await;
        
        match result {
            Ok(response) => {
                println!("Success: {} results", response.results.len());
            }
            Err(mcp_core::error::SearchError::RateLimited { engine }) => {
                println!("Rate limited by {}", engine);
            }
            Err(mcp_core::error::SearchError::Blocked { engine }) => {
                println!("Blocked by {}", engine);
            }
            Err(mcp_core::error::SearchError::Parse(msg)) => {
                println!("Parse error: {}", msg);
            }
            Err(e) => {
                println!("Other error: {}", e);
            }
        }
    }
}
