//! Background Runtime Workers
//!
//! Lightweight background workers for intelligence updates.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::context_memory::ContextMemoryEngine;
use crate::intelligence::health::WorkspaceHealthEngine;
use crate::intelligence::recommendation::RecommendationEngine;
use crate::predictive::engine::PredictiveEngine;
use crate::predictive::workflow::WorkflowEngine;
use crate::runtime::cache::IntelligenceCache;
use crate::runtime::emitter::IntelligenceEmitter;
use crate::runtime::shutdown::ShutdownCoordinator;

/// Background runtime workers for intelligence systems.
pub struct RuntimeWorkers {
    emitter: IntelligenceEmitter,
    cache: IntelligenceCache,
    predictive_engine: PredictiveEngine,
    #[allow(dead_code)]
    workflow_engine: WorkflowEngine,
    health_engine: WorkspaceHealthEngine,
    recommendation_engine: RecommendationEngine,
    context_memory_engine: ContextMemoryEngine,
    active_workspace: Arc<RwLock<Option<Uuid>>>,
    shutdown: ShutdownCoordinator,
    handles: Arc<RwLock<Vec<JoinHandle<()>>>>,
}

impl RuntimeWorkers {
    pub fn new(
        emitter: IntelligenceEmitter,
        cache: IntelligenceCache,
        predictive_engine: PredictiveEngine,
        workflow_engine: WorkflowEngine,
        health_engine: WorkspaceHealthEngine,
        recommendation_engine: RecommendationEngine,
        context_memory_engine: ContextMemoryEngine,
    ) -> Self {
        Self {
            emitter,
            cache,
            predictive_engine,
            workflow_engine,
            health_engine,
            recommendation_engine,
            context_memory_engine,
            active_workspace: Arc::new(RwLock::new(None)),
            shutdown: ShutdownCoordinator::new(),
            handles: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Sets the currently active workspace.
    pub async fn set_active_workspace(&self, workspace_id: Option<Uuid>) {
        let mut active = self.active_workspace.write().await;
        *active = workspace_id;
    }

    /// Starts all background workers.
    pub fn start(self: Arc<Self>) {
        let mut handles = Vec::new();

        // Prediction update worker - runs every 2 minutes
        {
            let workers = Arc::clone(&self);
            let mut shutdown_rx = workers.shutdown.subscribe();
            let handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(120));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if let Err(e) = workers.update_predictions().await {
                                tracing::warn!("Prediction update failed: {}", e);
                            }
                        }
                        _ = shutdown_rx.recv() => {
                            tracing::info!("Prediction worker shutting down");
                            break;
                        }
                    }
                }
            });
            handles.push(handle);
        }

        // Workflow detection worker - runs every 30 seconds
        {
            let workers = Arc::clone(&self);
            let mut shutdown_rx = workers.shutdown.subscribe();
            let handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if let Err(e) = workers.detect_workflow().await {
                                tracing::warn!("Workflow detection failed: {}", e);
                            }
                        }
                        _ = shutdown_rx.recv() => {
                            tracing::info!("Workflow worker shutting down");
                            break;
                        }
                    }
                }
            });
            handles.push(handle);
        }

        // Health recalculation worker - runs every 5 minutes
        {
            let workers = Arc::clone(&self);
            let mut shutdown_rx = workers.shutdown.subscribe();
            let handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(300));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if let Err(e) = workers.recalculate_health().await {
                                tracing::warn!("Health recalculation failed: {}", e);
                            }
                        }
                        _ = shutdown_rx.recv() => {
                            tracing::info!("Health worker shutting down");
                            break;
                        }
                    }
                }
            });
            handles.push(handle);
        }

        // Recommendation update worker - runs every 3 minutes
        {
            let workers = Arc::clone(&self);
            let mut shutdown_rx = workers.shutdown.subscribe();
            let handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(180));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if let Err(e) = workers.update_recommendations().await {
                                tracing::warn!("Recommendation update failed: {}", e);
                            }
                        }
                        _ = shutdown_rx.recv() => {
                            tracing::info!("Recommendation worker shutting down");
                            break;
                        }
                    }
                }
            });
            handles.push(handle);
        }

        // Context snapshot worker - runs every 10 minutes
        {
            let workers = Arc::clone(&self);
            let mut shutdown_rx = workers.shutdown.subscribe();
            let handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(600));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if let Err(e) = workers.create_snapshot().await {
                                tracing::warn!("Context snapshot failed: {}", e);
                            }
                        }
                        _ = shutdown_rx.recv() => {
                            tracing::info!("Snapshot worker shutting down");
                            break;
                        }
                    }
                }
            });
            handles.push(handle);
        }

        // Store handles for shutdown
        let handles_lock = self.handles.clone();
        tokio::spawn(async move {
            let mut handles_guard = handles_lock.write().await;
            *handles_guard = handles;
        });
    }

    /// Initiates graceful shutdown of all workers.
    pub async fn shutdown(&self) {
        tracing::info!("Initiating runtime workers shutdown");
        self.shutdown.shutdown();

        // Wait for all workers to complete
        let mut handles = self.handles.write().await;
        for handle in handles.drain(..) {
            let _ = handle.await;
        }

        tracing::info!("All runtime workers stopped");
    }

    /// Updates predictions for the active workspace.
    async fn update_predictions(&self) -> Result<(), Box<dyn std::error::Error>> {
        let workspace_id = self.active_workspace.read().await;
        if let Some(workspace_id) = *workspace_id {
            let predictions = self.predictive_engine.get_predictions_summary().await?;
            self.cache
                .set_predictions("global".to_string(), predictions);
            self.emitter.emit_prediction_updated(Some(workspace_id));
        }
        Ok(())
    }

    /// Detects current workflow for the active workspace.
    async fn detect_workflow(&self) -> Result<(), Box<dyn std::error::Error>> {
        let workspace_id = self.active_workspace.read().await;
        if let Some(_workspace_id) = *workspace_id {
            // Workflow detection happens in real-time via event processing
            // Workers just maintain cache - no active detection needed here
        }
        Ok(())
    }

    /// Recalculates health for the active workspace.
    async fn recalculate_health(&self) -> Result<(), Box<dyn std::error::Error>> {
        let workspace_id = self.active_workspace.read().await;
        if let Some(workspace_id) = *workspace_id {
            let health = self.health_engine.calculate_health(workspace_id).await?;
            self.cache
                .set_health_score(workspace_id, health.overall_score);
            self.emitter
                .emit_health_updated(workspace_id, health.overall_score);
        }
        Ok(())
    }

    /// Updates recommendations for the active workspace.
    async fn update_recommendations(&self) -> Result<(), Box<dyn std::error::Error>> {
        let workspace_id = self.active_workspace.read().await;
        if let Some(workspace_id) = *workspace_id {
            let recommendations = self
                .recommendation_engine
                .generate_recommendations(workspace_id)
                .await?;
            let count = recommendations.len();
            self.cache
                .set_recommendations(workspace_id, recommendations);
            self.emitter
                .emit_recommendation_updated(workspace_id, count);
        }
        Ok(())
    }

    /// Creates a context snapshot for the active workspace.
    async fn create_snapshot(&self) -> Result<(), Box<dyn std::error::Error>> {
        let workspace_id = self.active_workspace.read().await;
        if let Some(workspace_id) = *workspace_id {
            use crate::context_memory::models::CreateSnapshotRequest;

            let request = CreateSnapshotRequest {
                workspace_id: workspace_id.to_string(),
                snapshot_type: crate::context_memory::models::SnapshotType::Auto,
                active_files: vec![],
                session_summary: None,
                timeline_references: None,
                analytics_summary: None,
                health_score: None,
                recommendations_summary: None,
                metadata: Some(serde_json::json!({"trigger": "background_worker"})),
            };

            let snapshot = self.context_memory_engine.create_snapshot(request).await?;
            self.emitter
                .emit_snapshot_created(workspace_id, snapshot.id);
        }
        Ok(())
    }

    /// Triggers immediate cache invalidation and updates for a workspace.
    pub async fn invalidate_and_update(&self, workspace_id: Uuid) {
        self.cache.invalidate_workspace(workspace_id);

        // Trigger immediate updates
        if let Err(e) = self.update_predictions().await {
            tracing::warn!("Immediate prediction update failed: {}", e);
        }
        if let Err(e) = self.update_recommendations().await {
            tracing::warn!("Immediate recommendation update failed: {}", e);
        }
    }

    /// Triggers global cache invalidation.
    pub fn invalidate_all(&self) {
        self.cache.invalidate_all();
    }
}
