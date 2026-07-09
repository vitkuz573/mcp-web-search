use mcp_core::error::{SearchError, SearchOptions, SearchResponse};
use mcp_search::SearchAggregator;
use mcp_search::engines::load_custom_engines;
use moka::future::Cache;
use reqwest::{Client, Proxy};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

use crate::analytics::SearchAnalytics;

/// Proxy configuration for HTTP requests
#[derive(Debug, Clone, Default)]
pub struct ProxyConfig {
    /// HTTP proxy URL (e.g., "http://proxy:8080")
    pub http_proxy: Option<String>,
    /// HTTPS proxy URL (e.g., "http://proxy:8080")
    pub https_proxy: Option<String>,
    /// SOCKS5 proxy URL (e.g., "socks5://proxy:1080")
    pub socks5_proxy: Option<String>,
    /// No proxy list (comma-separated hostnames)
    pub no_proxy: Option<String>,
    /// Per-engine proxy overrides (engine_name -> proxy_url)
    pub engine_proxies: std::collections::HashMap<String, String>,
}

impl ProxyConfig {
    /// Create proxy config from environment variables
    pub fn from_env() -> Self {
        Self {
            http_proxy: std::env::var("HTTP_PROXY")
                .or_else(|_| std::env::var("http_proxy"))
                .ok(),
            https_proxy: std::env::var("HTTPS_PROXY")
                .or_else(|_| std::env::var("https_proxy"))
                .ok(),
            socks5_proxy: std::env::var("ALL_PROXY")
                .or_else(|_| std::env::var("all_proxy"))
                .ok(),
            no_proxy: std::env::var("NO_PROXY")
                .or_else(|_| std::env::var("no_proxy"))
                .ok(),
            engine_proxies: std::collections::HashMap::new(),
        }
    }

    /// Create proxy config from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, SearchError> {
        #[derive(serde::Deserialize)]
        struct ProxyConfigToml {
            #[serde(default)]
            http_proxy: Option<String>,
            #[serde(default)]
            https_proxy: Option<String>,
            #[serde(default)]
            socks5_proxy: Option<String>,
            #[serde(default)]
            no_proxy: Option<String>,
            #[serde(default)]
            engine_proxies: std::collections::HashMap<String, String>,
        }

        let toml_config: ProxyConfigToml = toml::from_str(toml_str)
            .map_err(|e| SearchError::Config(format!("Failed to parse proxy config: {}", e)))?;

        Ok(Self {
            http_proxy: toml_config.http_proxy,
            https_proxy: toml_config.https_proxy,
            socks5_proxy: toml_config.socks5_proxy,
            no_proxy: toml_config.no_proxy,
            engine_proxies: toml_config.engine_proxies,
        })
    }

    /// Get proxy URL for a specific engine (falls back to global proxy)
    pub fn proxy_for_engine(&self, engine_name: &str) -> Option<&str> {
        if let Some(proxy) = self.engine_proxies.get(engine_name) {
            return Some(proxy.as_str());
        }

        self.https_proxy
            .as_deref()
            .or(self.http_proxy.as_deref())
            .or(self.socks5_proxy.as_deref())
    }
}

pub struct AppState {
    pub aggregator: SearchAggregator,
    pub cache: Cache<String, SearchResponse>,
    pub proxy_config: ProxyConfig,
    pub analytics: SearchAnalytics,
}

impl AppState {
    pub fn new() -> Self {
        Self::with_config(None, None)
    }

    pub fn with_config(
        proxy_config: Option<ProxyConfig>,
        plugin_dir: Option<&PathBuf>,
    ) -> Self {
        let proxy_cfg = proxy_config.unwrap_or_default();

        let mut builder = Client::builder()
            .timeout(Duration::from_secs(10));

        if let Some(ref http_proxy) = proxy_cfg.http_proxy {
            if let Ok(proxy) = Proxy::http(http_proxy) {
                info!("Using HTTP proxy: {}", http_proxy);
                builder = builder.proxy(proxy);
            }
        }
        if let Some(ref https_proxy) = proxy_cfg.https_proxy {
            if let Ok(proxy) = Proxy::https(https_proxy) {
                info!("Using HTTPS proxy: {}", https_proxy);
                builder = builder.proxy(proxy);
            }
        }
        if let Some(ref socks5) = proxy_cfg.socks5_proxy {
            if let Ok(proxy) = Proxy::all(socks5) {
                info!("Using SOCKS5 proxy: {}", socks5);
                builder = builder.proxy(proxy);
            }
        }

        let client = builder.build().expect("Failed to create HTTP client");

        let mut aggregator = SearchAggregator::new();

        let engine_names = vec!["google", "duckduckgo", "bing", "brave", "youtube", "yahoo"];
        for name in engine_names {
            let engine_client = if let Some(proxy_url) = proxy_cfg.proxy_for_engine(name) {
                Self::build_proxied_client(proxy_url)
                    .unwrap_or_else(|e| {
                        warn!("Failed to build proxied client for {}: {}", name, e);
                        client.clone()
                    })
            } else {
                client.clone()
            };

            if let Some(engine) = mcp_search::create_engine(name, engine_client) {
                info!("Registered search engine: {}", name);
                let engine: Arc<dyn mcp_core::traits::SearchEngine> = Arc::from(engine);
                aggregator.add_engine(engine);
            }
        }

        if let Some(dir) = plugin_dir {
            let custom_engines = load_custom_engines(dir, client.clone());
            for engine in custom_engines {
                let engine_name = engine.name().to_string();
                info!("Registered custom engine: {}", engine_name);
                let engine: Arc<dyn mcp_core::traits::SearchEngine> = Arc::from(engine);
                aggregator.add_engine(engine);
            }
        }

        let cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(3600))
            .build();

        Self {
            aggregator,
            cache,
            proxy_config: proxy_cfg,
            analytics: SearchAnalytics::new(),
        }
    }

    fn build_proxied_client(proxy_url: &str) -> Result<Client, reqwest::Error> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(10));

        if proxy_url.starts_with("socks5://") {
            builder = builder.proxy(Proxy::all(proxy_url)?);
        } else if proxy_url.starts_with("https://") {
            builder = builder.proxy(Proxy::https(proxy_url)?);
        } else {
            builder = builder.proxy(Proxy::http(proxy_url)?);
        }

        builder.build()
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
            self.analytics.record_cache_hit();
            return Ok(cached);
        }

        self.analytics.record_cache_miss();

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
