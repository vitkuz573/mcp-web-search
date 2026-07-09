use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::collections::HashMap;
use std::sync::RwLock;
use serde::Serialize;
use rmcp::schemars;

/// Per-engine analytics
#[derive(Debug, Default)]
struct EngineAnalytics {
    total_searches: AtomicI64,
    total_latency_ms: AtomicU64,
    total_results: AtomicI64,
    errors: AtomicI64,
}

/// Global search analytics
pub struct SearchAnalytics {
    engines: RwLock<HashMap<String, EngineAnalytics>>,
    total_searches: AtomicI64,
    total_results: AtomicI64,
    cache_hits: AtomicI64,
    cache_misses: AtomicI64,
    deduped_results: AtomicI64,
}

impl SearchAnalytics {
    pub fn new() -> Self {
        Self {
            engines: RwLock::new(HashMap::new()),
            total_searches: AtomicI64::new(0),
            total_results: AtomicI64::new(0),
            cache_hits: AtomicI64::new(0),
            cache_misses: AtomicI64::new(0),
            deduped_results: AtomicI64::new(0),
        }
    }

    pub fn record_search(&self, engine: &str, latency_ms: u64, results: usize) {
        self.total_searches.fetch_add(1, Ordering::Relaxed);
        self.total_results.fetch_add(results as i64, Ordering::Relaxed);

        let mut engines = self.engines.write().unwrap();
        let analytics = engines.entry(engine.to_string()).or_default();
        analytics.total_searches.fetch_add(1, Ordering::Relaxed);
        analytics.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
        analytics.total_results.fetch_add(results as i64, Ordering::Relaxed);
    }

    pub fn record_error(&self, engine: &str) {
        let mut engines = self.engines.write().unwrap();
        let analytics = engines.entry(engine.to_string()).or_default();
        analytics.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_dedup(&self, count: i64) {
        self.deduped_results.fetch_add(count, Ordering::Relaxed);
    }

    /// Returns (avg_latency_ms, success_rate, total_searches)
    pub fn engine_stats(&self, engine: &str) -> (f64, f64, i64) {
        let engines = self.engines.read().unwrap();
        if let Some(analytics) = engines.get(engine) {
            let searches = analytics.total_searches.load(Ordering::Relaxed);
            let latency = analytics.total_latency_ms.load(Ordering::Relaxed);
            let errors = analytics.errors.load(Ordering::Relaxed);

            if searches == 0 {
                return (0.0, 1.0, 0);
            }

            let avg_latency = latency as f64 / searches as f64;
            let success_rate = (searches - errors) as f64 / searches as f64;
            (avg_latency, success_rate, searches)
        } else {
            (0.0, 1.0, 0)
        }
    }

    pub fn snapshot(&self) -> SearchStatsSnapshot {
        let total_searches = self.total_searches.load(Ordering::Relaxed);
        let total_results = self.total_results.load(Ordering::Relaxed);
        let cache_hits = self.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.cache_misses.load(Ordering::Relaxed);
        let deduped = self.deduped_results.load(Ordering::Relaxed);

        let engines_used: Vec<String> = {
            let engines = self.engines.read().unwrap();
            engines.keys().cloned().collect()
        };

        // Calculate avg latency across all engines
        let (total_latency, total_engine_searches) = {
            let engines = self.engines.read().unwrap();
            let mut total_latency = 0u64;
            let mut total_searches = 0i64;
            for analytics in engines.values() {
                total_latency += analytics.total_latency_ms.load(Ordering::Relaxed);
                total_searches += analytics.total_searches.load(Ordering::Relaxed);
            }
            (total_latency, total_searches)
        };

        let avg_latency = if total_engine_searches > 0 {
            total_latency as f64 / total_engine_searches as f64
        } else {
            0.0
        };

        SearchStatsSnapshot {
            total_searches,
            total_results,
            avg_latency_ms: avg_latency,
            engines_used,
            cache_hits,
            cache_misses,
            deduped_results: deduped,
        }
    }
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct SearchStatsSnapshot {
    pub total_searches: i64,
    pub total_results: i64,
    pub avg_latency_ms: f64,
    pub engines_used: Vec<String>,
    pub cache_hits: i64,
    pub cache_misses: i64,
    pub deduped_results: i64,
}
