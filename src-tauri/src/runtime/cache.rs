//! Intelligence Cache
//!
//! Smart caching for predictions, recommendations, and health metrics
//! with automatic invalidation.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use crate::intelligence::recommendation::Recommendation;
use crate::predictive::models::PredictionsSummary;

/// Cache entry with expiration tracking.
#[derive(Clone, Debug)]
struct CacheEntry<T> {
    value: T,
    created_at: DateTime<Utc>,
    ttl_seconds: i64,
}

impl<T> CacheEntry<T> {
    fn new(value: T, ttl_seconds: i64) -> Self {
        Self {
            value,
            created_at: Utc::now(),
            ttl_seconds,
        }
    }

    fn is_expired(&self) -> bool {
        let age = Utc::now().signed_duration_since(self.created_at);
        age.num_seconds() > self.ttl_seconds
    }
}

/// Intelligence cache with smart invalidation.
#[derive(Clone)]
pub struct IntelligenceCache {
    predictions: Arc<RwLock<HashMap<String, CacheEntry<PredictionsSummary>>>>,
    recommendations: Arc<RwLock<HashMap<Uuid, CacheEntry<Vec<Recommendation>>>>>,
    health_scores: Arc<RwLock<HashMap<Uuid, CacheEntry<f64>>>>,
}

impl IntelligenceCache {
    pub fn new() -> Self {
        Self {
            predictions: Arc::new(RwLock::new(HashMap::new())),
            recommendations: Arc::new(RwLock::new(HashMap::new())),
            health_scores: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Gets cached predictions summary.
    pub fn get_predictions(&self, key: &str) -> Option<PredictionsSummary> {
        let cache = self.predictions.read().ok()?;
        let entry = cache.get(key)?;
        if entry.is_expired() {
            drop(cache);
            self.invalidate_predictions(key);
            None
        } else {
            Some(entry.value.clone())
        }
    }

    /// Caches predictions summary with TTL of 5 minutes.
    pub fn set_predictions(&self, key: String, predictions: PredictionsSummary) {
        if let Ok(mut cache) = self.predictions.write() {
            cache.insert(key, CacheEntry::new(predictions, 300));
        }
    }

    /// Invalidates predictions cache for a specific key.
    pub fn invalidate_predictions(&self, key: &str) {
        if let Ok(mut cache) = self.predictions.write() {
            cache.remove(key);
        }
    }

    /// Invalidates all predictions cache.
    pub fn invalidate_all_predictions(&self) {
        if let Ok(mut cache) = self.predictions.write() {
            cache.clear();
        }
    }

    /// Gets cached recommendations for a workspace.
    pub fn get_recommendations(&self, workspace_id: Uuid) -> Option<Vec<Recommendation>> {
        let cache = self.recommendations.read().ok()?;
        let entry = cache.get(&workspace_id)?;
        if entry.is_expired() {
            drop(cache);
            self.invalidate_recommendations(workspace_id);
            None
        } else {
            Some(entry.value.clone())
        }
    }

    /// Caches recommendations with TTL of 10 minutes.
    pub fn set_recommendations(&self, workspace_id: Uuid, recommendations: Vec<Recommendation>) {
        if let Ok(mut cache) = self.recommendations.write() {
            cache.insert(workspace_id, CacheEntry::new(recommendations, 600));
        }
    }

    /// Invalidates recommendations cache for a workspace.
    pub fn invalidate_recommendations(&self, workspace_id: Uuid) {
        if let Ok(mut cache) = self.recommendations.write() {
            cache.remove(&workspace_id);
        }
    }

    /// Invalidates all recommendations cache.
    pub fn invalidate_all_recommendations(&self) {
        if let Ok(mut cache) = self.recommendations.write() {
            cache.clear();
        }
    }

    /// Gets cached health score for a workspace.
    pub fn get_health_score(&self, workspace_id: Uuid) -> Option<f64> {
        let cache = self.health_scores.read().ok()?;
        let entry = cache.get(&workspace_id)?;
        if entry.is_expired() {
            drop(cache);
            self.invalidate_health_score(workspace_id);
            None
        } else {
            Some(entry.value)
        }
    }

    /// Caches health score with TTL of 15 minutes.
    pub fn set_health_score(&self, workspace_id: Uuid, health_score: f64) {
        if let Ok(mut cache) = self.health_scores.write() {
            cache.insert(workspace_id, CacheEntry::new(health_score, 900));
        }
    }

    /// Invalidates health score cache for a workspace.
    pub fn invalidate_health_score(&self, workspace_id: Uuid) {
        if let Ok(mut cache) = self.health_scores.write() {
            cache.remove(&workspace_id);
        }
    }

    /// Invalidates all health score cache.
    pub fn invalidate_all_health_scores(&self) {
        if let Ok(mut cache) = self.health_scores.write() {
            cache.clear();
        }
    }

    /// Invalidates all caches for a workspace (called on workspace switch, file changes, etc).
    pub fn invalidate_workspace(&self, workspace_id: Uuid) {
        self.invalidate_recommendations(workspace_id);
        self.invalidate_health_score(workspace_id);
        self.invalidate_all_predictions();
    }

    /// Invalidates all caches (called on graph updates, session changes, etc).
    pub fn invalidate_all(&self) {
        self.invalidate_all_predictions();
        self.invalidate_all_recommendations();
        self.invalidate_all_health_scores();
    }

    /// Number of entries currently held across all cache segments
    /// (RC-10 M1 diagnostics surface).
    pub fn entry_count(&self) -> usize {
        let predictions = self.predictions.read().map(|c| c.len()).unwrap_or(0);
        let recommendations = self.recommendations.read().map(|c| c.len()).unwrap_or(0);
        let health_scores = self.health_scores.read().map(|c| c.len()).unwrap_or(0);
        predictions + recommendations + health_scores
    }
}

impl Default for IntelligenceCache {
    fn default() -> Self {
        Self::new()
    }
}
