//! Automation Engine for rule-based workflow automation.

use chrono::{Timelike, Utc};
use uuid::Uuid;

use crate::context_memory::ContextMemoryEngine;
use crate::errors::DatabaseError;
use crate::intelligence::recommendation::RecommendationEngine;
use crate::predictive::models::{
    ActionType, AutomationExecution, AutomationRule, CreateAutomationRuleRequest, TriggerType,
};
use crate::predictive::repository::PredictiveRepository;
use crate::repositories::{FileRepository, WorkspaceRepository};

/// Automation engine for executing rule-based actions.
#[derive(Clone)]
pub struct AutomationEngine {
    repository: PredictiveRepository,
    workspace_repo: WorkspaceRepository,
    file_repo: FileRepository,
    context_memory_engine: ContextMemoryEngine,
    recommendation_engine: RecommendationEngine,
}

impl AutomationEngine {
    pub fn new(
        repository: PredictiveRepository,
        workspace_repo: WorkspaceRepository,
        file_repo: FileRepository,
        context_memory_engine: ContextMemoryEngine,
        recommendation_engine: RecommendationEngine,
    ) -> Self {
        Self {
            repository,
            workspace_repo,
            file_repo,
            context_memory_engine,
            recommendation_engine,
        }
    }

    /// Creates a new automation rule.
    pub async fn create_rule(
        &self,
        request: CreateAutomationRuleRequest,
    ) -> Result<AutomationRule, DatabaseError> {
        self.repository.create_automation_rule(request).await
    }

    /// Lists all automation rules.
    pub async fn list_rules(&self) -> Result<Vec<AutomationRule>, DatabaseError> {
        self.repository.list_automation_rules().await
    }

    /// Updates a rule's enabled status.
    pub async fn update_rule_enabled(
        &self,
        rule_id: i64,
        enabled: bool,
    ) -> Result<(), DatabaseError> {
        self.repository.update_rule_enabled(rule_id, enabled).await
    }

    /// Deletes a rule.
    pub async fn delete_rule(&self, rule_id: i64) -> Result<(), DatabaseError> {
        self.repository.delete_automation_rule(rule_id).await
    }

    /// Evaluates and executes matching rules for a trigger.
    pub async fn evaluate_trigger(
        &self,
        trigger_type: TriggerType,
        context: serde_json::Value,
    ) -> Result<Vec<AutomationExecution>, DatabaseError> {
        let rules = self.list_rules().await?;
        let mut executions = Vec::new();

        for rule in rules {
            if !rule.enabled || rule.trigger_type != trigger_type {
                continue;
            }

            // Check if trigger condition matches
            if self.evaluate_trigger_condition(&rule, &context).await? {
                // Execute action
                match self.execute_action(&rule, &context).await {
                    Ok(result) => {
                        let execution = self
                            .repository
                            .log_automation_execution(rule.id, true, result)
                            .await?;
                        executions.push(execution);
                    }
                    Err(e) => {
                        let error_result = serde_json::json!({
                            "error": e.to_string()
                        });
                        let execution = self
                            .repository
                            .log_automation_execution(rule.id, false, error_result)
                            .await?;
                        executions.push(execution);
                    }
                }
            }
        }

        Ok(executions)
    }

    /// Evaluates if a trigger condition is met.
    async fn evaluate_trigger_condition(
        &self,
        rule: &AutomationRule,
        context: &serde_json::Value,
    ) -> Result<bool, DatabaseError> {
        match rule.trigger_type {
            TriggerType::WorkspaceActivated => {
                // Always true when workspace is activated
                Ok(true)
            }
            TriggerType::LongInactive => {
                // Check if inactive duration exceeds threshold
                if let Some(threshold) = rule.trigger_config.get("threshold_seconds") {
                    if let Some(inactive_seconds) = context.get("inactive_seconds") {
                        return Ok(inactive_seconds.as_i64().unwrap_or(0)
                            >= threshold.as_i64().unwrap_or(1800));
                    }
                }
                Ok(false)
            }
            TriggerType::DuplicatesExceedThreshold => {
                // Check if duplicate count exceeds threshold
                if let Some(threshold) = rule.trigger_config.get("threshold") {
                    if let Some(duplicate_count) = context.get("duplicate_count") {
                        return Ok(duplicate_count.as_i64().unwrap_or(0)
                            >= threshold.as_i64().unwrap_or(10));
                    }
                }
                Ok(false)
            }
            TriggerType::ProductivityDrop => {
                // Check if productivity score dropped below threshold
                if let Some(threshold) = rule.trigger_config.get("threshold") {
                    if let Some(productivity_score) = context.get("productivity_score") {
                        return Ok(productivity_score.as_f64().unwrap_or(100.0)
                            < threshold.as_f64().unwrap_or(50.0));
                    }
                }
                Ok(false)
            }
            TriggerType::WorkflowTransition => {
                // Check if workflow transitioned
                Ok(context.get("workflow_transition").is_some())
            }
            TriggerType::TimeOfDay => {
                // Check if current hour matches
                if let Some(target_hour) = rule.trigger_config.get("hour") {
                    let current_hour = Utc::now().hour() as i64;
                    return Ok(current_hour == target_hour.as_i64().unwrap_or(-1));
                }
                Ok(false)
            }
        }
    }

    /// Executes an action.
    async fn execute_action(
        &self,
        rule: &AutomationRule,
        context: &serde_json::Value,
    ) -> Result<serde_json::Value, DatabaseError> {
        match rule.action_type {
            ActionType::RestoreContext => {
                // Restore previous context from snapshot
                if let Some(workspace_id_str) = context.get("workspace_id").and_then(|v| v.as_str())
                {
                    let snapshot = self
                        .context_memory_engine
                        .get_latest_snapshot(workspace_id_str)
                        .await?;

                    Ok(serde_json::json!({
                        "action": "restore_context",
                        "snapshot_found": snapshot.is_some(),
                    }))
                } else {
                    Ok(serde_json::json!({
                        "action": "restore_context",
                        "error": "No workspace_id in context"
                    }))
                }
            }
            ActionType::CreateSnapshot => {
                // Create a snapshot
                if let Some(workspace_id_str) = context.get("workspace_id").and_then(|v| v.as_str())
                {
                    if let Ok(workspace_uuid) = Uuid::parse_str(workspace_id_str) {
                        let files = self.file_repo.list_by_workspace(workspace_uuid).await?;
                        let active_files: Vec<String> =
                            files.iter().map(|f| f.path_or_url.clone()).collect();

                        let snapshot = self
                            .context_memory_engine
                            .auto_snapshot(workspace_uuid, active_files)
                            .await?;

                        return Ok(serde_json::json!({
                            "action": "create_snapshot",
                            "snapshot_id": snapshot.id,
                        }));
                    }
                }
                Ok(serde_json::json!({
                    "action": "create_snapshot",
                    "error": "Invalid workspace_id"
                }))
            }
            ActionType::RecommendCleanup => {
                // Generate cleanup recommendation
                Ok(serde_json::json!({
                    "action": "recommend_cleanup",
                    "message": "Consider cleaning up duplicate files and old artifacts"
                }))
            }
            ActionType::RecommendBreak => {
                // Generate break recommendation
                Ok(serde_json::json!({
                    "action": "recommend_break",
                    "message": "You've been working for a while. Consider taking a break."
                }))
            }
            ActionType::NotifyUser => {
                // Send notification
                let message = rule
                    .action_config
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Automation notification");

                Ok(serde_json::json!({
                    "action": "notify_user",
                    "message": message
                }))
            }
            ActionType::SwitchWorkspace => {
                // Switch to specified workspace
                if let Some(target_workspace_id) = rule
                    .action_config
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                {
                    Ok(serde_json::json!({
                        "action": "switch_workspace",
                        "target_workspace_id": target_workspace_id
                    }))
                } else {
                    Ok(serde_json::json!({
                        "action": "switch_workspace",
                        "error": "No target workspace_id specified"
                    }))
                }
            }
        }
    }

    /// Triggers automation on workspace activation.
    pub async fn on_workspace_activated(
        &self,
        workspace_id: Uuid,
    ) -> Result<Vec<AutomationExecution>, DatabaseError> {
        let context = serde_json::json!({
            "workspace_id": workspace_id.to_string()
        });

        self.evaluate_trigger(TriggerType::WorkspaceActivated, context)
            .await
    }

    /// Triggers automation on inactivity detection.
    pub async fn on_inactivity_detected(
        &self,
        inactive_seconds: i64,
    ) -> Result<Vec<AutomationExecution>, DatabaseError> {
        let context = serde_json::json!({
            "inactive_seconds": inactive_seconds
        });

        self.evaluate_trigger(TriggerType::LongInactive, context)
            .await
    }
}
