//! Intelligence Event Emitter
//!
//! Emits real-time events for all intelligence subsystems.

use serde::Serialize;
use uuid::Uuid;

use crate::app_events::{
    emit, AppEventEmitter, EVENT_ACTION_EXECUTED, EVENT_HEALTH_UPDATED, EVENT_PREDICTION_UPDATED,
    EVENT_RECOMMENDATION_UPDATED, EVENT_SESSION_ENDED, EVENT_SESSION_STARTED,
    EVENT_SNAPSHOT_CREATED, EVENT_WORKFLOW_CHANGED,
};
use crate::predictive::models::WorkflowType;

/// Intelligence event emitter for real-time frontend updates.
#[derive(Clone)]
pub struct IntelligenceEmitter {
    emitter: std::sync::Arc<dyn AppEventEmitter>,
}

impl IntelligenceEmitter {
    pub fn new(emitter: std::sync::Arc<dyn AppEventEmitter>) -> Self {
        Self { emitter }
    }

    /// Emits session started event.
    pub fn emit_session_started(&self, workspace_id: Uuid, session_id: String) {
        let payload = SessionStartedPayload {
            workspace_id: workspace_id.to_string(),
            session_id,
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        emit(self.emitter.as_ref(), EVENT_SESSION_STARTED, &payload);
    }

    /// Emits session ended event.
    pub fn emit_session_ended(
        &self,
        workspace_id: Uuid,
        session_id: String,
        duration_seconds: i64,
        productivity_score: Option<f64>,
    ) {
        let payload = SessionEndedPayload {
            workspace_id: workspace_id.to_string(),
            session_id,
            ended_at: chrono::Utc::now().to_rfc3339(),
            duration_seconds,
            productivity_score,
        };
        emit(self.emitter.as_ref(), EVENT_SESSION_ENDED, &payload);
    }

    /// Emits workflow changed event.
    pub fn emit_workflow_changed(
        &self,
        workspace_id: Uuid,
        workflow_type: WorkflowType,
        confidence: f64,
    ) {
        let payload = WorkflowChangedPayload {
            workspace_id: workspace_id.to_string(),
            workflow_type: format!("{:?}", workflow_type).to_lowercase(),
            confidence,
            detected_at: chrono::Utc::now().to_rfc3339(),
        };
        emit(self.emitter.as_ref(), EVENT_WORKFLOW_CHANGED, &payload);
    }

    /// Emits prediction updated event.
    pub fn emit_prediction_updated(&self, workspace_id: Option<Uuid>) {
        let payload = PredictionUpdatedPayload {
            workspace_id: workspace_id.map(|id| id.to_string()),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        emit(self.emitter.as_ref(), EVENT_PREDICTION_UPDATED, &payload);
    }

    /// Emits recommendation updated event.
    pub fn emit_recommendation_updated(&self, workspace_id: Uuid, recommendation_count: usize) {
        let payload = RecommendationUpdatedPayload {
            workspace_id: workspace_id.to_string(),
            recommendation_count,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        emit(
            self.emitter.as_ref(),
            EVENT_RECOMMENDATION_UPDATED,
            &payload,
        );
    }

    /// Emits health updated event.
    pub fn emit_health_updated(&self, workspace_id: Uuid, health_score: f64) {
        let payload = HealthUpdatedPayload {
            workspace_id: workspace_id.to_string(),
            health_score,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        emit(self.emitter.as_ref(), EVENT_HEALTH_UPDATED, &payload);
    }

    /// Emits snapshot created event.
    pub fn emit_snapshot_created(&self, workspace_id: Uuid, snapshot_id: i64) {
        let payload = SnapshotCreatedPayload {
            workspace_id: workspace_id.to_string(),
            snapshot_id,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        emit(self.emitter.as_ref(), EVENT_SNAPSHOT_CREATED, &payload);
    }

    /// Emits action executed event.
    pub fn emit_action_executed(
        &self,
        workspace_id: Uuid,
        action_type: String,
        success: bool,
        error_message: Option<String>,
    ) {
        let payload = ActionExecutedPayload {
            workspace_id: workspace_id.to_string(),
            action_type,
            success,
            error_message,
            executed_at: chrono::Utc::now().to_rfc3339(),
        };
        emit(self.emitter.as_ref(), EVENT_ACTION_EXECUTED, &payload);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionStartedPayload {
    workspace_id: String,
    session_id: String,
    started_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionEndedPayload {
    workspace_id: String,
    session_id: String,
    ended_at: String,
    duration_seconds: i64,
    productivity_score: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowChangedPayload {
    workspace_id: String,
    workflow_type: String,
    confidence: f64,
    detected_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PredictionUpdatedPayload {
    workspace_id: Option<String>,
    updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecommendationUpdatedPayload {
    workspace_id: String,
    recommendation_count: usize,
    updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthUpdatedPayload {
    workspace_id: String,
    health_score: f64,
    updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotCreatedPayload {
    workspace_id: String,
    snapshot_id: i64,
    created_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActionExecutedPayload {
    workspace_id: String,
    action_type: String,
    success: bool,
    error_message: Option<String>,
    executed_at: String,
}
