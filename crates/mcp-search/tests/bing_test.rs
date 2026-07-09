#[cfg(test)]
mod tests {
    use mcp_core::error::SearchOptions;
    use mcp_core::traits::SearchEngine;
    use mcp_search::engines::BingSearch;
    use reqwest::Client;

    #[tokio::test]
    async fn test_bing_search() {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create client");

        let engine = BingSearch::new(client);
        let options = SearchOptions {
            language: Some("en".to_string()),
            page_size: 10,
            ..Default::default()
        };

        let result = engine.search("rust programming language", &options).await;
        
        match result {
            Ok(response) => {
                println!("Bing search successful!");
                println!("Engine: {}", response.engine);
                println!("Results: {}", response.results.len());
                println!("Search time: {}ms", response.search_time_ms);
                
                for (i, result) in response.results.iter().enumerate() {
                    println!("\nResult {}:", i + 1);
                    println!("  Title: {}", result.title);
                    println!("  URL: {}", result.url);
                    println!("  Snippet: {}", result.snippet);
                }
                
                assert!(!response.results.is_empty(), "Should have at least one result");
                assert_eq!(response.engine, "bing");
            }
            Err(e) => {
                panic!("Bing search failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_bing_search_with_pagination() {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create client");

        let engine = BingSearch::new(client);
        let options = SearchOptions {
            language: Some("en".to_string()),
            page_size: 10,
            page: Some(1),
            ..Default::default()
        };

        let result = engine.search("rust programming language", &options).await;
        
        match result {
            Ok(response) => {
                println!("Bing page 2 search successful!");
                println!("Results: {}", response.results.len());
                assert!(!response.results.is_empty());
            }
            Err(e) => {
                panic!("Bing page 2 search failed: {}", e);
            }
        }
    }
}
