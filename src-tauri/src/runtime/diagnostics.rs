//! Runtime Diagnostics
//!
//! Provides comprehensive diagnostics for runtime health and performance.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::runtime::health::RuntimeHealthService;

/// Cache statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatistics {
    pub predictions_cached: bool,
    pub total_hits: u64,
    pub total_misses: u64,
    pub hit_rate: f64,
}

/// Worker status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerStatus {
    pub name: String,
    pub active: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub total_runs: u64,
    pub errors: u64,
}

/// Event throughput metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventMetrics {
    pub total_events: u64,
    pub events_per_second: f64,
    pub last_event: Option<DateTime<Utc>>,
}

/// Complete runtime diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDiagnostics {
    pub uptime_seconds: u64,
    pub cache_statistics: CacheStatistics,
    pub worker_status: Vec<WorkerStatus>,
    pub event_metrics: EventMetrics,
    pub health_status: String,
    pub timestamp: DateTime<Utc>,
}

/// Provides diagnostic information about the runtime.
pub struct DiagnosticsService {
    health_service: RuntimeHealthService,
}

impl DiagnosticsService {
    /// Creates a new diagnostics service.
    pub fn new(health_service: RuntimeHealthService) -> Self {
        Self { health_service }
    }

    /// Gets comprehensive runtime diagnostics.
    pub async fn get_diagnostics(&self) -> RuntimeDiagnostics {
        let health = self.health_service.get_health().await;

        let cache_statistics = CacheStatistics {
            predictions_cached: false, // TODO: check if predictions are cached
            total_hits: 0,             // Tracked by health service
            total_misses: 0,
            hit_rate: health.cache_hit_rate,
        };

        let worker_status = health
            .components
            .iter()
            .map(|c| WorkerStatus {
                name: c.name.clone(),
                active: true,
                last_run: c.last_execution,
                total_runs: c.execution_count,
                errors: c.error_count,
            })
            .collect();

        let event_metrics = EventMetrics {
            total_events: health.event_throughput,
            events_per_second: health.event_throughput as f64 / health.uptime_seconds as f64,
            last_event: None,
        };

        RuntimeDiagnostics {
            uptime_seconds: health.uptime_seconds,
            cache_statistics,
            worker_status,
            event_metrics,
            health_status: format!("{:?}", health.status),
            timestamp: Utc::now(),
        }
    }

    /// Gets a summary string for logging.
    pub async fn get_summary(&self) -> String {
        let diagnostics = self.get_diagnostics().await;
        format!(
            "Runtime: {} | Workers: {} | Cache Hit Rate: {:.2}% | Events: {} | Uptime: {}s",
            diagnostics.health_status,
            diagnostics.worker_status.len(),
            diagnostics.cache_statistics.hit_rate * 100.0,
            diagnostics.event_metrics.total_events,
            diagnostics.uptime_seconds
        )
    }
}
