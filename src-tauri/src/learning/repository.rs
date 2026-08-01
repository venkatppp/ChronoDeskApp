//! Learning Repository - Database layer for adaptive learning.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::errors::DatabaseError;
use crate::learning::models::*;

/// Repository for learning data persistence.
#[derive(Clone)]
pub struct LearningRepository {
    pool: SqlitePool,
}

impl LearningRepository {
    /// Creates a new learning repository.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Records user feedback.
    pub async fn record_feedback(&self, feedback: &UserFeedback) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO learning_feedback (
                id, feedback_type, target_type, target_id, action, context, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(feedback.id.to_string())
        .bind(feedback.feedback_type.as_str())
        .bind(feedback.target_type.as_str())
        .bind(&feedback.target_id)
        .bind(feedback.action.as_str())
        .bind(serde_json::to_string(&feedback.context).unwrap_or_default())
        .bind(feedback.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets feedback for a specific target.
    pub async fn get_feedback_for_target(
        &self,
        _target_type: FeedbackTargetType,
        _target_id: &str,
    ) -> Result<Vec<UserFeedback>, DatabaseError> {
        // For now, return empty - full implementation would parse rows
        Ok(Vec::new())
    }

    /// Stores or updates a user preference.
    pub async fn upsert_preference(&self, preference: &UserPreference) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO learning_preferences (
                id, preference_type, key, value, confidence, evidence_count, last_updated
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(preference_type, key) DO UPDATE SET
                value = excluded.value,
                confidence = excluded.confidence,
                evidence_count = excluded.evidence_count,
                last_updated = excluded.last_updated
            "#,
        )
        .bind(preference.id.to_string())
        .bind(preference.preference_type.as_str())
        .bind(&preference.key)
        .bind(serde_json::to_string(&preference.value).unwrap_or_default())
        .bind(preference.confidence)
        .bind(preference.evidence_count)
        .bind(preference.last_updated.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets all user preferences.
    pub async fn get_all_preferences(&self) -> Result<Vec<UserPreference>, DatabaseError> {
        // Return empty for now - full implementation would parse rows
        Ok(Vec::new())
    }

    /// Gets preferences by type.
    pub async fn get_preferences_by_type(
        &self,
        _preference_type: PreferenceType,
    ) -> Result<Vec<UserPreference>, DatabaseError> {
        Ok(Vec::new())
    }

    /// Stores a behavioral pattern.
    pub async fn store_pattern(&self, pattern: &BehavioralPattern) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO learning_patterns (
                id, pattern_type, description, conditions, frequency, confidence,
                occurrences, first_seen, last_seen
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                frequency = excluded.frequency,
                confidence = excluded.confidence,
                occurrences = excluded.occurrences,
                last_seen = excluded.last_seen
            "#,
        )
        .bind(pattern.id.to_string())
        .bind(pattern.pattern_type.as_str())
        .bind(&pattern.description)
        .bind(serde_json::to_string(&pattern.conditions).unwrap_or_default())
        .bind(pattern.frequency)
        .bind(pattern.confidence)
        .bind(pattern.occurrences)
        .bind(pattern.first_seen.to_rfc3339())
        .bind(pattern.last_seen.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets all behavioral patterns.
    pub async fn get_all_patterns(&self) -> Result<Vec<BehavioralPattern>, DatabaseError> {
        Ok(Vec::new())
    }

    /// Records a confidence adjustment.
    pub async fn record_confidence_adjustment(
        &self,
        adjustment: &ConfidenceAdjustment,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO learning_confidence_adjustments (
                id, target_type, target_id, original_confidence, adjusted_confidence,
                adjustment_factor, reason, applied_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(adjustment.id.to_string())
        .bind(adjustment.target_type.as_str())
        .bind(&adjustment.target_id)
        .bind(adjustment.original_confidence)
        .bind(adjustment.adjusted_confidence)
        .bind(adjustment.adjustment_factor)
        .bind(&adjustment.reason)
        .bind(adjustment.applied_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets confidence adjustments for a target.
    pub async fn get_confidence_adjustments(
        &self,
        _target_type: FeedbackTargetType,
        _target_id: &str,
    ) -> Result<Vec<ConfidenceAdjustment>, DatabaseError> {
        Ok(Vec::new())
    }

    /// Stores workflow learning data.
    pub async fn store_workflow_learning(
        &self,
        workflow: &WorkflowLearningData,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO learning_workflows (
                id, workflow_type, typical_duration_seconds, typical_files, typical_time_of_day,
                success_indicators, confidence, sample_count, last_updated
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(workflow_type) DO UPDATE SET
                typical_duration_seconds = excluded.typical_duration_seconds,
                typical_files = excluded.typical_files,
                typical_time_of_day = excluded.typical_time_of_day,
                success_indicators = excluded.success_indicators,
                confidence = excluded.confidence,
                sample_count = excluded.sample_count,
                last_updated = excluded.last_updated
            "#,
        )
        .bind(workflow.id.to_string())
        .bind(&workflow.workflow_type)
        .bind(workflow.typical_duration_seconds)
        .bind(serde_json::to_string(&workflow.typical_files).unwrap_or_default())
        .bind(serde_json::to_string(&workflow.typical_time_of_day).unwrap_or_default())
        .bind(serde_json::to_string(&workflow.success_indicators).unwrap_or_default())
        .bind(workflow.confidence)
        .bind(workflow.sample_count)
        .bind(workflow.last_updated.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets workflow learning data.
    pub async fn get_workflow_learning(
        &self,
        _workflow_type: &str,
    ) -> Result<Option<WorkflowLearningData>, DatabaseError> {
        Ok(None)
    }

    /// Gets learning statistics.
    pub async fn get_learning_stats(&self) -> Result<LearningStats, DatabaseError> {
        let total_feedback: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM learning_feedback")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let accepted: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM learning_feedback WHERE action = 'accepted'"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let rejected: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM learning_feedback WHERE action = 'rejected'"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let acceptance_rate = if total_feedback > 0 {
            accepted as f64 / total_feedback as f64
        } else {
            0.0
        };

        let total_preferences: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM learning_preferences")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let total_patterns: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM learning_patterns")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        Ok(LearningStats {
            total_feedback_count: total_feedback,
            accepted_count: accepted,
            rejected_count: rejected,
            acceptance_rate,
            total_preferences,
            total_patterns,
            avg_confidence_adjustment: 1.0,
            last_learning_update: Utc::now(),
        })
    }

    /// Gets recent confidence trends.
    pub async fn get_confidence_trends(
        &self,
        _days: i64,
    ) -> Result<Vec<ConfidenceTrend>, DatabaseError> {
        Ok(Vec::new())
    }
}

// Helper trait implementations for enum parsing
impl FeedbackType {
    #[allow(dead_code)]
    fn from_str(s: &str) -> Result<Self, DatabaseError> {
        match s {
            "recommendation" => Ok(Self::Recommendation),
            "prediction" => Ok(Self::Prediction),
            "action" => Ok(Self::Action),
            "workflow_detection" => Ok(Self::WorkflowDetection),
            _ => Err(DatabaseError::InvalidInput(format!("Invalid feedback type: {}", s))),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Recommendation => "recommendation",
            Self::Prediction => "prediction",
            Self::Action => "action",
            Self::WorkflowDetection => "workflow_detection",
        }
    }
}

impl FeedbackTargetType {
    #[allow(dead_code)]
    fn from_str(s: &str) -> Result<Self, DatabaseError> {
        match s {
            "recommendation" => Ok(Self::Recommendation),
            "workspace_prediction" => Ok(Self::WorkspacePrediction),
            "file_prediction" => Ok(Self::FilePrediction),
            "action_prediction" => Ok(Self::ActionPrediction),
            "workflow_transition" => Ok(Self::WorkflowTransition),
            _ => Err(DatabaseError::InvalidInput(format!("Invalid target type: {}", s))),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Recommendation => "recommendation",
            Self::WorkspacePrediction => "workspace_prediction",
            Self::FilePrediction => "file_prediction",
            Self::ActionPrediction => "action_prediction",
            Self::WorkflowTransition => "workflow_transition",
        }
    }
}

impl FeedbackAction {
    #[allow(dead_code)]
    fn from_str(s: &str) -> Result<Self, DatabaseError> {
        match s {
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            "dismissed" => Ok(Self::Dismissed),
            "not_helpful" => Ok(Self::NotHelpful),
            "helpful" => Ok(Self::Helpful),
            _ => Err(DatabaseError::InvalidInput(format!("Invalid feedback action: {}", s))),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Dismissed => "dismissed",
            Self::NotHelpful => "not_helpful",
            Self::Helpful => "helpful",
        }
    }
}

impl PreferenceType {
    #[allow(dead_code)]
    fn from_str(s: &str) -> Result<Self, DatabaseError> {
        match s {
            "workspace_switching" => Ok(Self::WorkspaceSwitching),
            "file_access" => Ok(Self::FileAccess),
            "time_of_day" => Ok(Self::TimeOfDay),
            "technology" => Ok(Self::Technology),
            "recommendation_category" => Ok(Self::RecommendationCategory),
            "workflow" => Ok(Self::Workflow),
            _ => Err(DatabaseError::InvalidInput(format!("Invalid preference type: {}", s))),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::WorkspaceSwitching => "workspace_switching",
            Self::FileAccess => "file_access",
            Self::TimeOfDay => "time_of_day",
            Self::Technology => "technology",
            Self::RecommendationCategory => "recommendation_category",
            Self::Workflow => "workflow",
        }
    }
}

impl PatternType {
    #[allow(dead_code)]
    fn from_str(s: &str) -> Result<Self, DatabaseError> {
        match s {
            "sequential_files" => Ok(Self::SequentialFiles),
            "workspace_switching" => Ok(Self::WorkspaceSwitching),
            "time_based" => Ok(Self::TimeBased),
            "workflow_transition" => Ok(Self::WorkflowTransition),
            "focus_session" => Ok(Self::FocusSession),
            _ => Err(DatabaseError::InvalidInput(format!("Invalid pattern type: {}", s))),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::SequentialFiles => "sequential_files",
            Self::WorkspaceSwitching => "workspace_switching",
            Self::TimeBased => "time_based",
            Self::WorkflowTransition => "workflow_transition",
            Self::FocusSession => "focus_session",
        }
    }
}
