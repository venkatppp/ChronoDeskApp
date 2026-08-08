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
        target_type: FeedbackTargetType,
        target_id: &str,
    ) -> Result<Vec<UserFeedback>, DatabaseError> {
        let rows: Vec<(String, String, String, String, String, String, String)> = sqlx::query_as(
            r#"
            SELECT id, feedback_type, target_type, target_id, action, context, created_at
            FROM learning_feedback
            WHERE target_type = ? AND target_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(target_type.as_str())
        .bind(target_id)
        .fetch_all(&self.pool)
        .await?;

        let mut feedback = Vec::new();
        for (id, feedback_type, target_type_str, target_id, action, context, created_at) in rows {
            feedback.push(UserFeedback {
                id: uuid::Uuid::parse_str(&id)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                feedback_type: FeedbackType::from_str(&feedback_type)?,
                target_type: FeedbackTargetType::from_str(&target_type_str)?,
                target_id,
                action: FeedbackAction::from_str(&action)?,
                context: serde_json::from_str(&context).unwrap_or_default(),
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&chrono::Utc),
            });
        }

        Ok(feedback)
    }

    /// Stores or updates a user preference.
    pub async fn upsert_preference(
        &self,
        preference: &UserPreference,
    ) -> Result<(), DatabaseError> {
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
        type PreferenceRow = (String, String, String, String, f64, i32, String);
        let rows: Vec<PreferenceRow> = sqlx::query_as(
            r#"
            SELECT id, preference_type, key, value, confidence, evidence_count, last_updated
            FROM learning_preferences
            ORDER BY confidence DESC, last_updated DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut preferences = Vec::new();
        for (id, preference_type, key, value, confidence, evidence_count, last_updated) in rows {
            preferences.push(UserPreference {
                id: uuid::Uuid::parse_str(&id)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                preference_type: PreferenceType::from_str(&preference_type)?,
                key,
                value: serde_json::from_str(&value).unwrap_or_default(),
                confidence,
                evidence_count,
                last_updated: chrono::DateTime::parse_from_rfc3339(&last_updated)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&chrono::Utc),
            });
        }

        Ok(preferences)
    }

    /// Gets preferences by type.
    pub async fn get_preferences_by_type(
        &self,
        preference_type: PreferenceType,
    ) -> Result<Vec<UserPreference>, DatabaseError> {
        type PreferenceRow = (String, String, String, String, f64, i32, String);
        let rows: Vec<PreferenceRow> = sqlx::query_as(
            r#"
            SELECT id, preference_type, key, value, confidence, evidence_count, last_updated
            FROM learning_preferences
            WHERE preference_type = ?
            ORDER BY confidence DESC
            "#,
        )
        .bind(preference_type.as_str())
        .fetch_all(&self.pool)
        .await?;

        let mut preferences = Vec::new();
        for (id, preference_type_str, key, value, confidence, evidence_count, last_updated) in rows
        {
            preferences.push(UserPreference {
                id: uuid::Uuid::parse_str(&id)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                preference_type: PreferenceType::from_str(&preference_type_str)?,
                key,
                value: serde_json::from_str(&value).unwrap_or_default(),
                confidence,
                evidence_count,
                last_updated: chrono::DateTime::parse_from_rfc3339(&last_updated)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&chrono::Utc),
            });
        }

        Ok(preferences)
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
        type PatternRow = (
            String,
            String,
            String,
            String,
            f64,
            f64,
            i32,
            String,
            String,
        );
        let rows: Vec<PatternRow> = sqlx::query_as(
            r#"
            SELECT id, pattern_type, description, conditions, frequency, confidence,
                   occurrences, first_seen, last_seen
            FROM learning_patterns
            ORDER BY confidence DESC, frequency DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut patterns = Vec::new();
        for (
            id,
            pattern_type,
            description,
            conditions,
            frequency,
            confidence,
            occurrences,
            first_seen,
            last_seen,
        ) in rows
        {
            patterns.push(BehavioralPattern {
                id: uuid::Uuid::parse_str(&id)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                pattern_type: PatternType::from_str(&pattern_type)?,
                description,
                conditions: serde_json::from_str(&conditions).unwrap_or_default(),
                frequency,
                confidence,
                occurrences,
                first_seen: chrono::DateTime::parse_from_rfc3339(&first_seen)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&chrono::Utc),
                last_seen: chrono::DateTime::parse_from_rfc3339(&last_seen)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&chrono::Utc),
            });
        }

        Ok(patterns)
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
        target_type: FeedbackTargetType,
        target_id: &str,
    ) -> Result<Vec<ConfidenceAdjustment>, DatabaseError> {
        type AdjustmentRow = (String, String, String, f64, f64, f64, String, String);
        let rows: Vec<AdjustmentRow> = sqlx::query_as(
            r#"
            SELECT id, target_type, target_id, original_confidence, adjusted_confidence,
                   adjustment_factor, reason, applied_at
            FROM learning_confidence_adjustments
            WHERE target_type = ? AND target_id = ?
            ORDER BY applied_at DESC
            "#,
        )
        .bind(target_type.as_str())
        .bind(target_id)
        .fetch_all(&self.pool)
        .await?;

        let mut adjustments = Vec::new();
        for (
            id,
            target_type_str,
            target_id,
            original_confidence,
            adjusted_confidence,
            adjustment_factor,
            reason,
            applied_at,
        ) in rows
        {
            adjustments.push(ConfidenceAdjustment {
                id: uuid::Uuid::parse_str(&id)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                target_type: FeedbackTargetType::from_str(&target_type_str)?,
                target_id,
                original_confidence,
                adjusted_confidence,
                adjustment_factor,
                reason,
                applied_at: chrono::DateTime::parse_from_rfc3339(&applied_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&chrono::Utc),
            });
        }

        Ok(adjustments)
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
        workflow_type: &str,
    ) -> Result<Option<WorkflowLearningData>, DatabaseError> {
        type WorkflowRow = (
            String,
            String,
            i64,
            String,
            String,
            String,
            f64,
            i32,
            String,
        );
        let row: Option<WorkflowRow> = sqlx::query_as(
            r#"
            SELECT id, workflow_type, typical_duration_seconds, typical_files, typical_time_of_day,
                   success_indicators, confidence, sample_count, last_updated
            FROM learning_workflows
            WHERE workflow_type = ?
            "#,
        )
        .bind(workflow_type)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((
            id,
            workflow_type,
            typical_duration_seconds,
            typical_files,
            typical_time_of_day,
            success_indicators,
            confidence,
            sample_count,
            last_updated,
        )) = row
        {
            Ok(Some(WorkflowLearningData {
                id: uuid::Uuid::parse_str(&id)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                workflow_type,
                typical_duration_seconds,
                typical_files: serde_json::from_str(&typical_files).unwrap_or_default(),
                typical_time_of_day: serde_json::from_str(&typical_time_of_day).unwrap_or_default(),
                success_indicators: serde_json::from_str(&success_indicators).unwrap_or_default(),
                confidence,
                sample_count,
                last_updated: chrono::DateTime::parse_from_rfc3339(&last_updated)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&chrono::Utc),
            }))
        } else {
            Ok(None)
        }
    }

    /// Gets learning statistics.
    pub async fn get_learning_stats(&self) -> Result<LearningStats, DatabaseError> {
        let total_feedback: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM learning_feedback")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let accepted: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM learning_feedback WHERE action = 'accepted'")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

        let rejected: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM learning_feedback WHERE action = 'rejected'")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

        let acceptance_rate = if total_feedback > 0 {
            accepted as f64 / total_feedback as f64
        } else {
            0.0
        };

        let total_preferences: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM learning_preferences")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

        let total_patterns: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM learning_patterns")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        // Average adjustment factor across real confidence adjustments —
        // not a constant; 0.0 (with no rows) means "no adjustments yet".
        let avg_confidence_adjustment: f64 = sqlx::query_scalar(
            "SELECT COALESCE(AVG(adjustment_factor), 0.0)
             FROM learning_confidence_adjustments",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0.0);

        // Most recent learning-relevant event (feedback, preference
        // update, pattern observation, or confidence adjustment).
        let last_learning_update: Option<String> = sqlx::query_scalar(
            "SELECT MAX(latest) FROM (
                SELECT MAX(created_at) AS latest FROM learning_feedback
                UNION ALL
                SELECT MAX(last_updated) AS latest FROM learning_preferences
                UNION ALL
                SELECT MAX(last_seen) AS latest FROM learning_patterns
                UNION ALL
                SELECT MAX(applied_at) AS latest FROM learning_confidence_adjustments
             )",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(None);

        let last_learning_update = last_learning_update
            .and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|d| d.with_timezone(&chrono::Utc))
                    .ok()
            })
            .unwrap_or_else(Utc::now); // No activity yet: "now" is honest (nothing older exists)

        Ok(LearningStats {
            total_feedback_count: total_feedback,
            accepted_count: accepted,
            rejected_count: rejected,
            acceptance_rate,
            total_preferences,
            total_patterns,
            avg_confidence_adjustment,
            last_learning_update,
        })
    }

    /// Computes recommendation accuracy from real accepted/rejected
    /// feedback, grouped by the `category` field of the feedback context
    /// (defaulting to the target type when no category was recorded).
    ///
    /// Returns empty per-category data when no feedback exists — the
    /// caller must treat that as "insufficient data", never as a valid
    /// accuracy reading.
    pub async fn get_feedback_accuracy(&self) -> Result<Vec<CategoryAccuracy>, DatabaseError> {
        let rows: Vec<(Option<String>, String, i64)> = sqlx::query_as(
            r#"
            SELECT json_extract(context, '$.category'),
                   action,
                   COUNT(*)
            FROM learning_feedback
            WHERE action IN ('accepted', 'rejected', 'helpful', 'not_helpful')
            GROUP BY json_extract(context, '$.category'), action
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        use std::collections::BTreeMap;
        let mut by_category: BTreeMap<String, (i64, i64)> = BTreeMap::new();
        for (category, action, count) in rows {
            let key = category.unwrap_or_else(|| "General".to_string());
            let entry = by_category.entry(key).or_insert((0, 0));
            if matches!(action.as_str(), "accepted" | "helpful") {
                entry.0 += count;
            } else {
                entry.1 += count;
            }
        }

        Ok(by_category
            .into_iter()
            .map(|(category, (accepted, rejected))| CategoryAccuracy {
                category,
                total: accepted + rejected,
                accepted,
                accuracy: if accepted + rejected > 0 {
                    accepted as f64 / (accepted + rejected) as f64
                } else {
                    0.0
                },
            })
            .collect())
    }

    /// Gets recent confidence trends.
    pub async fn get_confidence_trends(
        &self,
        days: i64,
    ) -> Result<Vec<ConfidenceTrend>, DatabaseError> {
        let rows: Vec<(String, f64, i32)> = sqlx::query_as(
            r#"
            SELECT DATE(applied_at) as date,
                   AVG(adjusted_confidence) as avg_conf,
                   COUNT(*) as count
            FROM learning_confidence_adjustments
            WHERE applied_at >= datetime('now', ? || ' days')
            GROUP BY DATE(applied_at)
            ORDER BY date ASC
            "#,
        )
        .bind(format!("-{}", days))
        .fetch_all(&self.pool)
        .await?;

        let mut trends = Vec::new();
        for (date_str, avg_confidence, adjustment_count) in rows {
            let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map_err(|e| DatabaseError::IoError(e.to_string()))?
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| DatabaseError::IoError("Invalid time".to_string()))?
                .and_utc();

            trends.push(ConfidenceTrend {
                date,
                avg_confidence,
                adjustment_count,
            });
        }

        Ok(trends)
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
            _ => Err(DatabaseError::InvalidInput(format!(
                "Invalid feedback type: {}",
                s
            ))),
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
            _ => Err(DatabaseError::InvalidInput(format!(
                "Invalid target type: {}",
                s
            ))),
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
            _ => Err(DatabaseError::InvalidInput(format!(
                "Invalid feedback action: {}",
                s
            ))),
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
            _ => Err(DatabaseError::InvalidInput(format!(
                "Invalid preference type: {}",
                s
            ))),
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
            _ => Err(DatabaseError::InvalidInput(format!(
                "Invalid pattern type: {}",
                s
            ))),
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
