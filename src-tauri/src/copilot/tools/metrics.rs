use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Default)]
pub(crate) struct ToolMetricsCollector {
    invocations: AtomicU64,
    successes: AtomicU64,
    failures: AtomicU64,
    cancellations: AtomicU64,
    retries: AtomicU64,
    total_duration_ms: AtomicU64,
    duration_samples: AtomicU64,
}

impl ToolMetricsCollector {
    pub(crate) fn record_invocation(&self) {
        self.invocations.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_success(&self, duration: Duration) {
        self.successes.fetch_add(1, Ordering::Relaxed);
        self.record_duration(duration);
    }

    pub(crate) fn record_failure(&self, duration: Duration) {
        self.failures.fetch_add(1, Ordering::Relaxed);
        self.record_duration(duration);
    }

    pub(crate) fn record_cancelled(&self, duration: Duration) {
        self.cancellations.fetch_add(1, Ordering::Relaxed);
        self.record_duration(duration);
    }

    pub(crate) fn record_retry(&self) {
        self.retries.fetch_add(1, Ordering::Relaxed);
    }

    fn record_duration(&self, duration: Duration) {
        self.total_duration_ms
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
        self.duration_samples.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn diagnostics(&self, registered_tools: usize) -> ToolDiagnostics {
        let samples = self.duration_samples.load(Ordering::Relaxed);
        let total_duration = self.total_duration_ms.load(Ordering::Relaxed);
        let invocations = self.invocations.load(Ordering::Relaxed);
        let successes = self.successes.load(Ordering::Relaxed);

        ToolDiagnostics {
            registered_tools,
            total_invocations: invocations,
            successful_invocations: successes,
            failed_invocations: self.failures.load(Ordering::Relaxed),
            cancelled_invocations: self.cancellations.load(Ordering::Relaxed),
            retried_invocations: self.retries.load(Ordering::Relaxed),
            average_duration_ms: if samples == 0 {
                0.0
            } else {
                total_duration as f64 / samples as f64
            },
            success_rate: if invocations == 0 {
                1.0
            } else {
                successes as f64 / invocations as f64
            },
        }
    }
}

/// Aggregate tool framework diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDiagnostics {
    pub registered_tools: usize,
    pub total_invocations: u64,
    pub successful_invocations: u64,
    pub failed_invocations: u64,
    pub cancelled_invocations: u64,
    pub retried_invocations: u64,
    pub average_duration_ms: f64,
    pub success_rate: f64,
}
