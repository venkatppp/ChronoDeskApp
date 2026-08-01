//! Workflow Engine for workflow detection and automation.

use chrono::Utc;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::predictive::models::{WorkflowState, WorkflowTransition, WorkflowType};
use crate::repositories::{FileRepository, TimelineRepository};
use crate::services::ContextService;

/// Workflow engine for detecting and managing user workflows.
#[derive(Clone)]
pub struct WorkflowEngine {
    timeline_repo: TimelineRepository,
    file_repo: FileRepository,
    #[allow(dead_code)]
    context_service: ContextService,
}

impl WorkflowEngine {
    pub fn new(
        timeline_repo: TimelineRepository,
        file_repo: FileRepository,
        context_service: ContextService,
    ) -> Self {
        Self {
            timeline_repo,
            file_repo,
            context_service,
        }
    }

    /// Detects the current workflow for a workspace.
    pub async fn detect_current_workflow(
        &self,
        workspace_id: Uuid,
    ) -> Result<Option<WorkflowState>, DatabaseError> {
        // Get recent timeline events
        let events = self
            .timeline_repo
            .list_by_workspace(workspace_id, Some(30))
            .await?;

        if events.is_empty() {
            return Ok(None);
        }

        // Get active files
        let files = self.file_repo.list_by_workspace(workspace_id).await?;
        let active_files: Vec<String> = files.iter().map(|f| f.path_or_url.clone()).collect();

        // Analyze patterns to detect workflow
        let workflow_type = self.infer_workflow_type(&events, &active_files)?;
        let confidence = self.calculate_workflow_confidence(&events, workflow_type)?;

        let started_at = events
            .first()
            .map(|e| e.occurred_at)
            .unwrap_or_else(Utc::now);

        Ok(Some(WorkflowState {
            workflow_type,
            started_at,
            workspace_id: workspace_id.to_string(),
            confidence,
            active_files: active_files.into_iter().take(10).collect(),
        }))
    }

    /// Detects workflow transitions.
    pub async fn detect_workflow_transition(
        &self,
        workspace_id: Uuid,
        previous_workflow: WorkflowType,
    ) -> Result<Option<WorkflowTransition>, DatabaseError> {
        let current = self.detect_current_workflow(workspace_id).await?;

        if let Some(current_state) = current {
            if current_state.workflow_type != previous_workflow && current_state.confidence > 0.6 {
                return Ok(Some(WorkflowTransition {
                    from_workflow: previous_workflow,
                    to_workflow: current_state.workflow_type,
                    confidence: current_state.confidence,
                    detected_at: Utc::now(),
                }));
            }
        }

        Ok(None)
    }

    /// Infers workflow type from patterns.
    fn infer_workflow_type(
        &self,
        events: &[crate::models::TimelineEvent],
        active_files: &[String],
    ) -> Result<WorkflowType, DatabaseError> {
        let mut coding_score: f64 = 0.0;
        let mut debugging_score: f64 = 0.0;
        let mut documentation_score: f64 = 0.0;
        let research_score: f64 = 0.0;
        let meeting_score: f64 = 0.0;

        // Analyze file extensions
        for file_path in active_files {
            let ext = std::path::Path::new(file_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            match ext {
                "rs" | "js" | "ts" | "py" | "java" | "go" | "cpp" | "c" => coding_score += 1.0,
                "md" | "txt" | "pdf" | "html" => documentation_score += 1.0,
                "log" | "json" | "xml" => debugging_score += 0.5,
                _ => {}
            }
        }

        // Analyze event patterns
        let edit_count = events
            .iter()
            .filter(|e| e.event_type.as_str() == "file_modified")
            .count();
        let create_count = events
            .iter()
            .filter(|e| e.event_type.as_str() == "file_created")
            .count();

        if edit_count > 10 {
            coding_score += 2.0;
        }
        if create_count > 3 {
            coding_score += 1.0;
        }

        // Check for documentation files
        if active_files
            .iter()
            .any(|f| f.contains("README") || f.contains("CHANGELOG") || f.contains("docs"))
        {
            documentation_score += 2.0;
        }

        // Check for test files (debugging)
        if active_files
            .iter()
            .any(|f| f.contains("test") || f.contains("spec"))
        {
            debugging_score += 2.0;
        }

        // Determine workflow type
        let max_score = coding_score
            .max(debugging_score)
            .max(documentation_score)
            .max(research_score)
            .max(meeting_score);

        if max_score == 0.0 {
            return Ok(WorkflowType::Custom);
        }

        if (coding_score - max_score).abs() < f64::EPSILON {
            Ok(WorkflowType::Coding)
        } else if (debugging_score - max_score).abs() < f64::EPSILON {
            Ok(WorkflowType::Debugging)
        } else if (documentation_score - max_score).abs() < f64::EPSILON {
            Ok(WorkflowType::Documentation)
        } else if (research_score - max_score).abs() < f64::EPSILON {
            Ok(WorkflowType::Research)
        } else if (meeting_score - max_score).abs() < f64::EPSILON {
            Ok(WorkflowType::Meeting)
        } else {
            Ok(WorkflowType::Custom)
        }
    }

    /// Calculates confidence in workflow detection.
    fn calculate_workflow_confidence(
        &self,
        events: &[crate::models::TimelineEvent],
        _workflow_type: WorkflowType,
    ) -> Result<f64, DatabaseError> {
        // Simple heuristic: more events = higher confidence
        let event_count = events.len();
        let confidence = (event_count as f64 / 30.0).clamp(0.5, 0.95);
        Ok(confidence)
    }
}
