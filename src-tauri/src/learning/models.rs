//! Adaptive Learning models and types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User feedback on a recommendation or prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeedback {
    pub id: Uuid,
    pub feedback_type: FeedbackType,
    pub target_type: FeedbackTargetType,
    pub target_id: String,
    pub action: FeedbackAction,
    pub context: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Type of feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    Recommendation,
    Prediction,
    Action,
    WorkflowDetection,
}

/// Target of feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackTargetType {
    Recommendation,
    WorkspacePrediction,
    FilePrediction,
    ActionPrediction,
    WorkflowTransition,
}

/// User action on feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackAction {
    /// User accepted/followed the recommendation.
    Accepted,

    /// User explicitly rejected the recommendation.
    Rejected,

    /// User dismissed without acting.
    Dismissed,

    /// User marked as not helpful.
    NotHelpful,

    /// User marked as helpful.
    Helpful,
}

/// Personal preference learned from user behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreference {
    pub id: Uuid,
    pub preference_type: PreferenceType,
    pub key: String,
    pub value: serde_json::Value,
    pub confidence: f64,
    pub evidence_count: i32,
    pub last_updated: DateTime<Utc>,
}

/// Type of preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceType {
    /// Preferred workspace switching patterns.
    WorkspaceSwitching,

    /// File opening patterns.
    FileAccess,

    /// Time-of-day preferences.
    TimeOfDay,

    /// Technology/language preferences.
    Technology,

    /// Recommendation category preferences.
    RecommendationCategory,

    /// Workflow preferences.
    Workflow,
}

/// Behavioral pattern learned from history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralPattern {
    pub id: Uuid,
    pub pattern_type: PatternType,
    pub description: String,
    pub conditions: serde_json::Value,
    pub frequency: f64,
    pub confidence: f64,
    pub occurrences: i32,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

/// Type of behavioral pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    /// Sequential file access pattern.
    SequentialFiles,

    /// Workspace switching pattern.
    WorkspaceSwitching,

    /// Time-based pattern.
    TimeBased,

    /// Workflow transition pattern.
    WorkflowTransition,

    /// Focus session pattern.
    FocusSession,
}

/// Confidence adjustment for predictions/recommendations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceAdjustment {
    pub id: Uuid,
    pub target_type: FeedbackTargetType,
    pub target_id: String,
    pub original_confidence: f64,
    pub adjusted_confidence: f64,
    pub adjustment_factor: f64,
    pub reason: String,
    pub applied_at: DateTime<Utc>,
}

/// Learning statistics and metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStats {
    pub total_feedback_count: i64,
    pub accepted_count: i64,
    pub rejected_count: i64,
    pub acceptance_rate: f64,
    pub total_preferences: i64,
    pub total_patterns: i64,
    pub avg_confidence_adjustment: f64,
    pub last_learning_update: DateTime<Utc>,
}

/// Learning insights for dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningInsights {
    pub stats: LearningStats,
    pub top_preferences: Vec<UserPreference>,
    pub recent_patterns: Vec<BehavioralPattern>,
    pub confidence_trends: Vec<ConfidenceTrend>,
    pub recommendation_accuracy: RecommendationAccuracy,
}

/// Confidence trend over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceTrend {
    pub date: DateTime<Utc>,
    pub avg_confidence: f64,
    pub adjustment_count: i32,
}

/// Recommendation accuracy metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationAccuracy {
    pub category_accuracy: Vec<CategoryAccuracy>,
    pub overall_accuracy: f64,
    pub total_recommendations: i64,
}

/// Accuracy per recommendation category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryAccuracy {
    pub category: String,
    pub accuracy: f64,
    pub total: i64,
    pub accepted: i64,
}

/// Request to submit user feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitFeedbackRequest {
    pub feedback_type: FeedbackType,
    pub target_type: FeedbackTargetType,
    pub target_id: String,
    pub action: FeedbackAction,
    pub context: Option<serde_json::Value>,
}

/// Workflow learning data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLearningData {
    pub id: Uuid,
    pub workflow_type: String,
    pub typical_duration_seconds: i64,
    pub typical_files: Vec<String>,
    pub typical_time_of_day: Vec<i32>,
    pub success_indicators: serde_json::Value,
    pub confidence: f64,
    pub sample_count: i32,
    pub last_updated: DateTime<Utc>,
}

/// Explanation for confidence change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceExplanation {
    pub target_id: String,
    pub target_type: String,
    pub original_confidence: f64,
    pub adjusted_confidence: f64,
    pub reasons: Vec<ExplanationReason>,
    pub timestamp: DateTime<Utc>,
}

/// Individual reason for confidence adjustment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationReason {
    pub factor: String,
    pub impact: f64,
    pub description: String,
}
