//! Repository for predictive intelligence data.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::errors::DatabaseError;
use crate::predictive::models::{
    ActionType, AutomationExecution, AutomationRule, CreateAutomationRuleRequest, LearningProfile,
    TriggerType,
};

/// Repository for predictive intelligence persistence.
#[derive(Clone)]
pub struct PredictiveRepository {
    pool: SqlitePool,
}

impl PredictiveRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Stores or updates a learning profile.
    pub async fn upsert_learning_profile(
        &self,
        profile: &LearningProfile,
    ) -> Result<(), DatabaseError> {
        let preferred_hours_json = serde_json::to_string(&profile.preferred_work_hours)?;
        let tech_prefs_json = serde_json::to_string(&profile.technology_preferences)?;
        let focus_pattern_json = serde_json::to_string(&profile.focus_patterns)?;

        sqlx::query(
            r#"
            INSERT INTO learning_profiles (
                user_id, preferred_work_hours, avg_session_duration_seconds,
                workspace_switch_frequency, technology_preferences, focus_patterns, last_updated
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id) DO UPDATE SET
                preferred_work_hours = excluded.preferred_work_hours,
                avg_session_duration_seconds = excluded.avg_session_duration_seconds,
                workspace_switch_frequency = excluded.workspace_switch_frequency,
                technology_preferences = excluded.technology_preferences,
                focus_patterns = excluded.focus_patterns,
                last_updated = excluded.last_updated
            "#,
        )
        .bind(&profile.user_id)
        .bind(&preferred_hours_json)
        .bind(profile.avg_session_duration_seconds)
        .bind(profile.workspace_switch_frequency)
        .bind(&tech_prefs_json)
        .bind(&focus_pattern_json)
        .bind(profile.last_updated)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets the learning profile for a user.
    pub async fn get_learning_profile(
        &self,
        user_id: &str,
    ) -> Result<Option<LearningProfile>, DatabaseError> {
        let row: Option<(String, String, i64, f64, String, String, String)> = sqlx::query_as(
            r#"
            SELECT user_id, preferred_work_hours, avg_session_duration_seconds,
                   workspace_switch_frequency, technology_preferences, focus_patterns, last_updated
            FROM learning_profiles
            WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((
            user_id,
            preferred_hours_json,
            avg_session_duration_seconds,
            workspace_switch_frequency,
            tech_prefs_json,
            focus_pattern_json,
            last_updated_str,
        )) = row
        {
            let preferred_work_hours = serde_json::from_str(&preferred_hours_json)?;
            let technology_preferences = serde_json::from_str(&tech_prefs_json)?;
            let focus_patterns = serde_json::from_str(&focus_pattern_json)?;
            let last_updated = chrono::DateTime::parse_from_rfc3339(&last_updated_str)
                .map_err(|e| DatabaseError::InvalidInput(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&Utc);

            Ok(Some(LearningProfile {
                user_id,
                preferred_work_hours,
                avg_session_duration_seconds,
                workspace_switch_frequency,
                technology_preferences,
                focus_patterns,
                last_updated,
            }))
        } else {
            Ok(None)
        }
    }

    /// Creates a new automation rule.
    pub async fn create_automation_rule(
        &self,
        request: CreateAutomationRuleRequest,
    ) -> Result<AutomationRule, DatabaseError> {
        let trigger_config_json = serde_json::to_string(&request.trigger_config)?;
        let action_config_json = serde_json::to_string(&request.action_config)?;
        let now = Utc::now();

        let result = sqlx::query(
            r#"
            INSERT INTO automation_rules (
                name, enabled, trigger_type, trigger_config, action_type, action_config, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&request.name)
        .bind(request.enabled)
        .bind(request.trigger_type.as_str())
        .bind(&trigger_config_json)
        .bind(request.action_type.as_str())
        .bind(&action_config_json)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_rowid();

        Ok(AutomationRule {
            id,
            name: request.name,
            enabled: request.enabled,
            trigger_type: request.trigger_type,
            trigger_config: request.trigger_config,
            action_type: request.action_type,
            action_config: request.action_config,
            created_at: now,
        })
    }

    /// Lists all automation rules.
    pub async fn list_automation_rules(&self) -> Result<Vec<AutomationRule>, DatabaseError> {
        let rows: Vec<(i64, String, bool, String, String, String, String, String)> =
            sqlx::query_as(
                r#"
            SELECT id, name, enabled, trigger_type, trigger_config, action_type, action_config, created_at
            FROM automation_rules
            ORDER BY created_at DESC
            "#,
            )
            .fetch_all(&self.pool)
            .await?;

        let mut rules = Vec::new();
        for (
            id,
            name,
            enabled,
            trigger_type_str,
            trigger_config_json,
            action_type_str,
            action_config_json,
            created_at_str,
        ) in rows
        {
            let trigger_type = self.parse_trigger_type(&trigger_type_str)?;
            let action_type = self.parse_action_type(&action_type_str)?;
            let trigger_config = serde_json::from_str(&trigger_config_json)?;
            let action_config = serde_json::from_str(&action_config_json)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| DatabaseError::InvalidInput(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&Utc);

            rules.push(AutomationRule {
                id,
                name,
                enabled,
                trigger_type,
                trigger_config,
                action_type,
                action_config,
                created_at,
            });
        }

        Ok(rules)
    }

    /// Updates an automation rule's enabled status.
    pub async fn update_rule_enabled(
        &self,
        rule_id: i64,
        enabled: bool,
    ) -> Result<(), DatabaseError> {
        sqlx::query("UPDATE automation_rules SET enabled = ? WHERE id = ?")
            .bind(enabled)
            .bind(rule_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Deletes an automation rule.
    pub async fn delete_automation_rule(&self, rule_id: i64) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM automation_rules WHERE id = ?")
            .bind(rule_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Logs an automation execution.
    pub async fn log_automation_execution(
        &self,
        rule_id: i64,
        success: bool,
        result: serde_json::Value,
    ) -> Result<AutomationExecution, DatabaseError> {
        let now = Utc::now();
        let result_json = serde_json::to_string(&result)?;

        let res = sqlx::query(
            r#"
            INSERT INTO automation_executions (rule_id, executed_at, success, result)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(rule_id)
        .bind(now)
        .bind(success)
        .bind(&result_json)
        .execute(&self.pool)
        .await?;

        let id = res.last_insert_rowid();

        Ok(AutomationExecution {
            id,
            rule_id,
            executed_at: now,
            success,
            result,
        })
    }

    fn parse_trigger_type(&self, s: &str) -> Result<TriggerType, DatabaseError> {
        match s {
            "workspace_activated" => Ok(TriggerType::WorkspaceActivated),
            "long_inactive" => Ok(TriggerType::LongInactive),
            "duplicates_exceed_threshold" => Ok(TriggerType::DuplicatesExceedThreshold),
            "productivity_drop" => Ok(TriggerType::ProductivityDrop),
            "workflow_transition" => Ok(TriggerType::WorkflowTransition),
            "time_of_day" => Ok(TriggerType::TimeOfDay),
            _ => Err(DatabaseError::InvalidInput(format!(
                "Unknown trigger type: {}",
                s
            ))),
        }
    }

    fn parse_action_type(&self, s: &str) -> Result<ActionType, DatabaseError> {
        match s {
            "restore_context" => Ok(ActionType::RestoreContext),
            "create_snapshot" => Ok(ActionType::CreateSnapshot),
            "recommend_cleanup" => Ok(ActionType::RecommendCleanup),
            "recommend_break" => Ok(ActionType::RecommendBreak),
            "notify_user" => Ok(ActionType::NotifyUser),
            "switch_workspace" => Ok(ActionType::SwitchWorkspace),
            _ => Err(DatabaseError::InvalidInput(format!(
                "Unknown action type: {}",
                s
            ))),
        }
    }
}
