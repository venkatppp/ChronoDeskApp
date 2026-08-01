//! Predictive Engine for intelligent predictions.

use chrono::{Timelike, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use crate::analytics::AnalyticsEngine;
use crate::context_memory::ContextMemoryEngine;
use crate::errors::DatabaseError;
use crate::predictive::models::{
    ActionPrediction, FilePrediction, PredictionsSummary, SessionContinuationPrediction,
    WorkspacePrediction,
};
use crate::repositories::{TimelineRepository, WorkspaceRepository};
use crate::services::ContextService;

/// Predictive engine for workspace, file, and action predictions.
#[derive(Clone)]
pub struct PredictiveEngine {
    workspace_repo: WorkspaceRepository,
    timeline_repo: TimelineRepository,
    context_service: ContextService,
    analytics_engine: AnalyticsEngine,
    context_memory_engine: ContextMemoryEngine,
}

impl PredictiveEngine {
    pub fn new(
        workspace_repo: WorkspaceRepository,
        timeline_repo: TimelineRepository,
        context_service: ContextService,
        analytics_engine: AnalyticsEngine,
        context_memory_engine: ContextMemoryEngine,
    ) -> Self {
        Self {
            workspace_repo,
            timeline_repo,
            context_service,
            analytics_engine,
            context_memory_engine,
        }
    }

    /// Predicts the next workspace the user will switch to.
    pub async fn predict_next_workspace(
        &self,
    ) -> Result<Option<WorkspacePrediction>, DatabaseError> {
        // Get all active workspaces
        let workspaces = self.workspace_repo.list_active_workspaces().await?;

        if workspaces.is_empty() {
            return Ok(None);
        }

        // Get the most recent workspace
        let mut workspace_scores: Vec<(String, String, f64, String)> = Vec::new();

        for workspace in &workspaces {
            let mut score = 0.0;
            let mut reasons = Vec::new();

            // Factor 1: Recent activity (most important)
            let recent_events = self
                .timeline_repo
                .list_by_workspace(workspace.id, Some(10))
                .await?;
            if !recent_events.is_empty() {
                let recency_score = 0.4;
                score += recency_score;
                reasons.push("recent activity".to_string());
            }

            // Factor 2: Time of day patterns and workspace activity
            let _current_hour = Utc::now().hour() as i32;
            if let Ok(insight) = self
                .analytics_engine
                .get_workspace_insight(workspace.id)
                .await
            {
                // Simple heuristic: if this workspace has significant activity
                if insight.total_sessions > 5 {
                    score += 0.2;
                    reasons.push("frequently used workspace".to_string());
                }

                // Factor for recent activity
                if insight.weekly_edits > 50 {
                    score += 0.2;
                    reasons.push("active this week".to_string());
                }
            }

            // Factor 3: Related workspaces
            let related = self
                .context_memory_engine
                .get_related_workspaces(&workspace.id.to_string(), 0.3, 5)
                .await?;
            if !related.is_empty() {
                score += 0.2;
                reasons.push("related to recent work".to_string());
            }

            workspace_scores.push((
                workspace.id.to_string(),
                workspace.name.clone(),
                score,
                reasons.join(", "),
            ));
        }

        // Sort by score
        workspace_scores.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

        if let Some((id, name, confidence, reason)) = workspace_scores.first() {
            if *confidence > 0.3 {
                return Ok(Some(WorkspacePrediction {
                    workspace_id: id.clone(),
                    workspace_name: name.clone(),
                    confidence: *confidence,
                    reason: reason.clone(),
                    predicted_at: Utc::now(),
                }));
            }
        }

        Ok(None)
    }

    /// Predicts the next files the user will open.
    pub async fn predict_next_files(
        &self,
        workspace_id: Uuid,
        limit: usize,
    ) -> Result<Vec<FilePrediction>, DatabaseError> {
        let mut predictions = Vec::new();

        // Get recent timeline events for this workspace
        let events = self
            .timeline_repo
            .list_by_workspace(workspace_id, Some(50))
            .await?;

        // Count file frequencies
        let mut file_counts: HashMap<String, i32> = HashMap::new();
        for event in &events {
            if let Some(_file_id) = event.file_id {
                // We'd need to look up the file path, but for now use a simple heuristic
                if let Some(metadata) = &event.metadata {
                    if let Some(path) = metadata.get("path").and_then(|v| v.as_str()) {
                        *file_counts.entry(path.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        // Convert to predictions
        let mut file_scores: Vec<(String, i32)> = file_counts.into_iter().collect();
        file_scores.sort_by_key(|a| std::cmp::Reverse(a.1));

        for (path, count) in file_scores.iter().take(limit) {
            let confidence = (*count as f64) / (events.len() as f64).max(1.0);
            predictions.push(FilePrediction {
                file_path: path.clone(),
                workspace_id: workspace_id.to_string(),
                confidence,
                reason: format!("opened {} times recently", count),
            });
        }

        Ok(predictions)
    }

    /// Predicts the next actions.
    pub async fn predict_next_actions(
        &self,
        limit: usize,
    ) -> Result<Vec<ActionPrediction>, DatabaseError> {
        let mut predictions = Vec::new();

        // Get recent timeline events to infer patterns
        let workspaces = self.workspace_repo.list_active_workspaces().await?;
        if workspaces.is_empty() {
            return Ok(predictions);
        }

        let most_recent = &workspaces[0];
        let events = self
            .timeline_repo
            .list_by_workspace(most_recent.id, Some(20))
            .await?;

        // Pattern 1: If many file edits, predict commit
        let edit_count = events
            .iter()
            .filter(|e| {
                e.event_type.as_str() == "file_modified" || e.event_type.as_str() == "file_created"
            })
            .count();

        if edit_count > 5 {
            predictions.push(ActionPrediction {
                action_type: "commit".to_string(),
                description: "Commit your changes".to_string(),
                confidence: (edit_count as f64 / 10.0).min(0.9),
                reason: format!("{} files modified", edit_count),
            });
        }

        // Pattern 2: Long session, predict break
        if let Some(session) = self
            .context_service
            .get_latest_workspace_session(most_recent.id)
            .await?
        {
            if session.duration_seconds > 7200 {
                // 2 hours
                predictions.push(ActionPrediction {
                    action_type: "take_break".to_string(),
                    description: "Take a break".to_string(),
                    confidence: 0.8,
                    reason: "Long focus session detected".to_string(),
                });
            }
        }

        // Pattern 3: Many open files, predict cleanup
        let file_count = events.iter().filter(|e| e.file_id.is_some()).count();
        if file_count > 20 {
            predictions.push(ActionPrediction {
                action_type: "cleanup".to_string(),
                description: "Clean up open files".to_string(),
                confidence: 0.7,
                reason: format!("{} files accessed recently", file_count),
            });
        }

        predictions.truncate(limit);
        Ok(predictions)
    }

    /// Predicts if the current session will continue.
    pub async fn predict_session_continuation(
        &self,
        workspace_id: Uuid,
    ) -> Result<Option<SessionContinuationPrediction>, DatabaseError> {
        // Get current session
        let session = self
            .context_service
            .get_latest_workspace_session(workspace_id)
            .await?;

        if let Some(sess) = session {
            // Get historical session durations
            let sessions = self
                .context_service
                .get_workspace_sessions(workspace_id, Some(10))
                .await?;

            if sessions.is_empty() {
                return Ok(None);
            }

            let avg_duration: f64 = sessions
                .iter()
                .map(|s| s.duration_seconds as f64)
                .sum::<f64>()
                / sessions.len() as f64;

            let current_duration = sess.duration_seconds as f64;
            let will_continue = current_duration < avg_duration * 0.8;
            let estimated_duration = (avg_duration - current_duration).max(0.0) as i64;

            let confidence = if will_continue { 0.7 } else { 0.6 };
            let reason = if will_continue {
                format!(
                    "Average session is {:.0} min, current is {:.0} min",
                    avg_duration / 60.0,
                    current_duration / 60.0
                )
            } else {
                "Session approaching typical duration".to_string()
            };

            return Ok(Some(SessionContinuationPrediction {
                will_continue,
                confidence,
                estimated_duration_seconds: estimated_duration,
                reason,
            }));
        }

        Ok(None)
    }

    /// Gets a predictions summary for the dashboard.
    pub async fn get_predictions_summary(&self) -> Result<PredictionsSummary, DatabaseError> {
        let next_workspace = self.predict_next_workspace().await?;

        let next_files = if let Some(ref ws) = next_workspace {
            if let Ok(workspace_uuid) = Uuid::parse_str(&ws.workspace_id) {
                self.predict_next_files(workspace_uuid, 5).await?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let next_actions = self.predict_next_actions(3).await?;

        let session_continuation = if let Some(ref ws) = next_workspace {
            if let Ok(workspace_uuid) = Uuid::parse_str(&ws.workspace_id) {
                self.predict_session_continuation(workspace_uuid).await?
            } else {
                None
            }
        } else {
            None
        };

        Ok(PredictionsSummary {
            next_workspace,
            next_files,
            next_actions,
            session_continuation,
            current_workflow: None, // Will be populated by WorkflowEngine
        })
    }
}
