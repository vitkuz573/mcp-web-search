use mcp_core::types::SelectorConfig;
use std::collections::HashMap;

pub struct SelectorSet {
    configs: HashMap<String, SelectorConfig>,
}

impl SelectorSet {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let configs: HashMap<String, SelectorConfig> = serde_json::from_str(json)?;
        Ok(Self { configs })
    }

    pub fn register(&mut self, name: String, config: SelectorConfig) {
        self.configs.insert(name, config);
    }

    pub fn get(&self, name: &str) -> Option<&SelectorConfig> {
        self.configs.get(name)
    }

    pub fn engine_names(&self) -> Vec<&str> {
        self.configs.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for SelectorSet {
    fn default() -> Self {
        let mut set = Self::new();

        set.register(
            "google".to_string(),
            SelectorConfig {
                search_url: "https://www.google.com/search?q={query}&hl={lang}".to_string(),
                result_container: "div.g".to_string(),
                title_selector: "h3".to_string(),
                url_selector: "a".to_string(),
                snippet_selector: "div.VwiC3b, span.aCOpRe".to_string(),
                date_selector: Some("span.LEwnzc, span.MUxGbd".to_string()),
                thumbnail_selector: None,
                pagination_selector: None,
                next_page_selector: Some("a#pnnext".to_string()),
            },
        );

        set.register(
            "duckduckgo".to_string(),
            SelectorConfig {
                search_url: "https://html.duckduckgo.com/html/?q={query}".to_string(),
                result_container: "div.result".to_string(),
                title_selector: "a.result__a".to_string(),
                url_selector: "a.result__a".to_string(),
                snippet_selector: "a.result__snippet".to_string(),
                date_selector: None,
                thumbnail_selector: None,
                pagination_selector: None,
                next_page_selector: Some("input[value='Next']".to_string()),
            },
        );

        set.register(
            "bing".to_string(),
            SelectorConfig {
                search_url: "https://www.bing.com/search?q={query}&setlang={lang}".to_string(),
                result_container: "li.b_algo".to_string(),
                title_selector: "h2 a".to_string(),
                url_selector: "h2 a".to_string(),
                snippet_selector: "div.b_caption p".to_string(),
                date_selector: None,
                thumbnail_selector: None,
                pagination_selector: None,
                next_page_selector: Some("a.sb_pagN".to_string()),
            },
        );

        set.register(
            "brave".to_string(),
            SelectorConfig {
                search_url: "https://search.brave.com/search?q={query}".to_string(),
                result_container: "div.snippet".to_string(),
                title_selector: "div.title a".to_string(),
                url_selector: "div.title a".to_string(),
                snippet_selector: "div.description".to_string(),
                date_selector: Some("time".to_string()),
                thumbnail_selector: Some("img.snippet-thumbnail".to_string()),
                pagination_selector: None,
                next_page_selector: Some("a.next".to_string()),
            },
        );

        set.register(
            "youtube".to_string(),
            SelectorConfig {
                search_url: "https://www.youtube.com/results?search_query={query}".to_string(),
                result_container: "div#contents ytd-video-renderer".to_string(),
                title_selector: "a#video-title".to_string(),
                url_selector: "a#video-title".to_string(),
                snippet_selector: "div#description-text".to_string(),
                date_selector: Some("span.ytd-video-meta-block".to_string()),
                thumbnail_selector: Some("img#img".to_string()),
                pagination_selector: None,
                next_page_selector: None,
            },
        );

        set.register(
            "yahoo".to_string(),
            SelectorConfig {
                search_url: "https://search.yahoo.com/search?p={query}".to_string(),
                result_container: "div.algo".to_string(),
                title_selector: "h3.title a".to_string(),
                url_selector: "h3.title a".to_string(),
                snippet_selector: "div.compText p".to_string(),
                date_selector: None,
                thumbnail_selector: None,
                pagination_selector: None,
                next_page_selector: Some("a.next".to_string()),
            },
        );

        set
    }
}
