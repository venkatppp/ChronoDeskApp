//! Runtime Configuration Settings
//!
//! Configurable settings for runtime behavior.

use serde::{Deserialize, Serialize};

/// Runtime configuration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSettings {
    /// Prediction update interval in seconds (default: 120)
    pub prediction_interval_secs: u64,

    /// Workflow detection interval in seconds (default: 30)
    pub workflow_interval_secs: u64,

    /// Health recalculation interval in seconds (default: 300)
    pub health_interval_secs: u64,

    /// Recommendation update interval in seconds (default: 180)
    pub recommendation_interval_secs: u64,

    /// Context snapshot interval in seconds (default: 600)
    pub snapshot_interval_secs: u64,

    /// Predictions cache TTL in seconds (default: 300)
    pub predictions_cache_ttl_secs: i64,

    /// Recommendations cache TTL in seconds (default: 600)
    pub recommendations_cache_ttl_secs: i64,

    /// Health scores cache TTL in seconds (default: 900)
    pub health_cache_ttl_secs: i64,

    /// Enable automatic recovery (default: true)
    pub enable_recovery: bool,

    /// Heartbeat interval in seconds (default: 60)
    pub heartbeat_interval_secs: u64,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            prediction_interval_secs: 120,
            workflow_interval_secs: 30,
            health_interval_secs: 300,
            recommendation_interval_secs: 180,
            snapshot_interval_secs: 600,
            predictions_cache_ttl_secs: 300,
            recommendations_cache_ttl_secs: 600,
            health_cache_ttl_secs: 900,
            enable_recovery: true,
            heartbeat_interval_secs: 60,
        }
    }
}

impl RuntimeSettings {
    /// Creates settings with custom values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets prediction interval.
    pub fn with_prediction_interval(mut self, secs: u64) -> Self {
        self.prediction_interval_secs = secs;
        self
    }

    /// Sets workflow interval.
    pub fn with_workflow_interval(mut self, secs: u64) -> Self {
        self.workflow_interval_secs = secs;
        self
    }

    /// Sets health interval.
    pub fn with_health_interval(mut self, secs: u64) -> Self {
        self.health_interval_secs = secs;
        self
    }

    /// Sets recommendation interval.
    pub fn with_recommendation_interval(mut self, secs: u64) -> Self {
        self.recommendation_interval_secs = secs;
        self
    }

    /// Sets snapshot interval.
    pub fn with_snapshot_interval(mut self, secs: u64) -> Self {
        self.snapshot_interval_secs = secs;
        self
    }

    /// Sets predictions cache TTL.
    pub fn with_predictions_cache_ttl(mut self, secs: i64) -> Self {
        self.predictions_cache_ttl_secs = secs;
        self
    }

    /// Sets recommendations cache TTL.
    pub fn with_recommendations_cache_ttl(mut self, secs: i64) -> Self {
        self.recommendations_cache_ttl_secs = secs;
        self
    }

    /// Sets health cache TTL.
    pub fn with_health_cache_ttl(mut self, secs: i64) -> Self {
        self.health_cache_ttl_secs = secs;
        self
    }

    /// Enables or disables automatic recovery.
    pub fn with_recovery(mut self, enable: bool) -> Self {
        self.enable_recovery = enable;
        self
    }

    /// Sets heartbeat interval.
    pub fn with_heartbeat_interval(mut self, secs: u64) -> Self {
        self.heartbeat_interval_secs = secs;
        self
    }

    /// Validates settings.
    pub fn validate(&self) -> Result<(), String> {
        if self.prediction_interval_secs < 10 {
            return Err("Prediction interval must be at least 10 seconds".to_string());
        }
        if self.workflow_interval_secs < 5 {
            return Err("Workflow interval must be at least 5 seconds".to_string());
        }
        if self.health_interval_secs < 60 {
            return Err("Health interval must be at least 60 seconds".to_string());
        }
        if self.recommendation_interval_secs < 60 {
            return Err("Recommendation interval must be at least 60 seconds".to_string());
        }
        if self.snapshot_interval_secs < 60 {
            return Err("Snapshot interval must be at least 60 seconds".to_string());
        }
        if self.predictions_cache_ttl_secs < 60 {
            return Err("Predictions cache TTL must be at least 60 seconds".to_string());
        }
        if self.recommendations_cache_ttl_secs < 60 {
            return Err("Recommendations cache TTL must be at least 60 seconds".to_string());
        }
        if self.health_cache_ttl_secs < 60 {
            return Err("Health cache TTL must be at least 60 seconds".to_string());
        }
        if self.heartbeat_interval_secs < 10 {
            return Err("Heartbeat interval must be at least 10 seconds".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_valid() {
        let settings = RuntimeSettings::default();
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn builder_pattern_works() {
        let settings = RuntimeSettings::new()
            .with_prediction_interval(60)
            .with_workflow_interval(15)
            .with_health_interval(120);

        assert_eq!(settings.prediction_interval_secs, 60);
        assert_eq!(settings.workflow_interval_secs, 15);
        assert_eq!(settings.health_interval_secs, 120);
    }

    #[test]
    fn validates_minimum_intervals() {
        let settings = RuntimeSettings::new().with_prediction_interval(5);
        assert!(settings.validate().is_err());

        let settings = RuntimeSettings::new().with_workflow_interval(2);
        assert!(settings.validate().is_err());
    }
}
