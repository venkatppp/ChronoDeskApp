//! Runtime Health Monitoring
//!
//! Tracks health and performance of runtime components.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::runtime::cache::IntelligenceCache;

/// Runtime health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Performance metrics for a runtime component.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentMetrics {
    pub name: String,
    pub status: HealthStatus,
    pub last_execution: Option<DateTime<Utc>>,
    pub execution_count: u64,
    pub error_count: u64,
    pub avg_execution_time_ms: f64,
}

/// Overall runtime health summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeHealth {
    pub status: HealthStatus,
    pub workers_active: usize,
    pub cache_hit_rate: f64,
    pub event_throughput: u64,
    pub uptime_seconds: u64,
    pub components: Vec<ComponentMetrics>,
    pub checked_at: DateTime<Utc>,
}

/// Tracks performance metrics for runtime components.
#[derive(Clone)]
pub struct RuntimeHealthService {
    #[allow(dead_code)]
    cache: IntelligenceCache,
    cache_hits: Arc<AtomicU64>,
    cache_misses: Arc<AtomicU64>,
    events_emitted: Arc<AtomicU64>,
    start_time: DateTime<Utc>,
    component_metrics: Arc<RwLock<Vec<ComponentMetrics>>>,
}

impl RuntimeHealthService {
    /// Creates a new runtime health service.
    pub fn new(cache: IntelligenceCache) -> Self {
        Self {
            cache,
            cache_hits: Arc::new(AtomicU64::new(0)),
            cache_misses: Arc::new(AtomicU64::new(0)),
            events_emitted: Arc::new(AtomicU64::new(0)),
            start_time: Utc::now(),
            component_metrics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Records a cache hit.
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a cache miss.
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Records an event emission.
    pub fn record_event(&self) {
        self.events_emitted.fetch_add(1, Ordering::Relaxed);
    }

    /// Calculates cache hit rate.
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Gets event throughput.
    pub fn event_throughput(&self) -> u64 {
        self.events_emitted.load(Ordering::Relaxed)
    }

    /// Gets runtime uptime in seconds.
    pub fn uptime_seconds(&self) -> u64 {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.start_time);
        duration.num_seconds() as u64
    }

    /// Updates metrics for a component.
    pub async fn update_component_metrics(&self, metrics: ComponentMetrics) {
        let mut components = self.component_metrics.write().await;

        // Find existing or insert new
        if let Some(existing) = components.iter_mut().find(|c| c.name == metrics.name) {
            *existing = metrics;
        } else {
            components.push(metrics);
        }
    }

    /// Gets overall runtime health.
    pub async fn get_health(&self) -> RuntimeHealth {
        let components = self.component_metrics.read().await.clone();

        // Determine overall status based on component health
        let status = if components
            .iter()
            .any(|c| matches!(c.status, HealthStatus::Unhealthy))
        {
            HealthStatus::Unhealthy
        } else if components
            .iter()
            .any(|c| matches!(c.status, HealthStatus::Degraded))
        {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        RuntimeHealth {
            status,
            workers_active: components.len(),
            cache_hit_rate: self.cache_hit_rate(),
            event_throughput: self.event_throughput(),
            uptime_seconds: self.uptime_seconds(),
            components,
            checked_at: Utc::now(),
        }
    }

    /// Resets all metrics (useful for testing).
    pub async fn reset(&self) {
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.events_emitted.store(0, Ordering::Relaxed);
        let mut components = self.component_metrics.write().await;
        components.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn initial_cache_hit_rate_is_zero() {
        let cache = IntelligenceCache::new();
        let service = RuntimeHealthService::new(cache);
        assert_eq!(service.cache_hit_rate(), 0.0);
    }

    #[tokio::test]
    async fn records_cache_hits_and_misses() {
        let cache = IntelligenceCache::new();
        let service = RuntimeHealthService::new(cache);

        service.record_cache_hit();
        service.record_cache_hit();
        service.record_cache_miss();

        assert_eq!(service.cache_hit_rate(), 2.0 / 3.0);
    }

    #[tokio::test]
    async fn tracks_event_throughput() {
        let cache = IntelligenceCache::new();
        let service = RuntimeHealthService::new(cache);

        service.record_event();
        service.record_event();
        service.record_event();

        assert_eq!(service.event_throughput(), 3);
    }

    #[tokio::test]
    async fn uptime_increases() {
        let cache = IntelligenceCache::new();
        let service = RuntimeHealthService::new(cache);

        tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;

        assert!(service.uptime_seconds() >= 1);
    }

    #[tokio::test]
    async fn health_status_reflects_components() {
        let cache = IntelligenceCache::new();
        let service = RuntimeHealthService::new(cache);

        service
            .update_component_metrics(ComponentMetrics {
                name: "test".to_string(),
                status: HealthStatus::Healthy,
                last_execution: Some(Utc::now()),
                execution_count: 1,
                error_count: 0,
                avg_execution_time_ms: 10.0,
            })
            .await;

        let health = service.get_health().await;
        assert!(matches!(health.status, HealthStatus::Healthy));
        assert_eq!(health.workers_active, 1);
    }
}
