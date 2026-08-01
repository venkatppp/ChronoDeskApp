//! Context Reasoning Engine
//!
//! Provides AI reasoning capabilities for context inference, workflow detection,
//! and recommendation explanations.

use chrono::Utc;
use uuid::Uuid;

use crate::context_memory::ContextMemoryEngine;
use crate::errors::DatabaseError;
use crate::intelligence::recommendation::RecommendationEngine;
use crate::predictive::engine::PredictiveEngine;
use crate::semantic::engine::SemanticMemoryEngine;
use crate::semantic::models::{
    Evidence, ExplainablePrediction, IndexDocumentRequest, SemanticDocumentType,
};
use crate::semantic::search::SemanticSearchEngine;

/// Context reasoning engine for AI inference and explanations.
#[derive(Clone)]
pub struct ContextReasoningEngine {
    semantic_engine: SemanticMemoryEngine,
    semantic_search: SemanticSearchEngine,
    predictive_engine: PredictiveEngine,
    recommendation_engine: RecommendationEngine,
    context_memory_engine: ContextMemoryEngine,
}

impl ContextReasoningEngine {
    /// Creates a new context reasoning engine.
    pub fn new(
        semantic_engine: SemanticMemoryEngine,
        semantic_search: SemanticSearchEngine,
        predictive_engine: PredictiveEngine,
        recommendation_engine: RecommendationEngine,
        context_memory_engine: ContextMemoryEngine,
    ) -> Self {
        Self {
            semantic_engine,
            semantic_search,
            predictive_engine,
            recommendation_engine,
            context_memory_engine,
        }
    }

    /// Infers related work based on a workspace.
    pub async fn infer_related_work(
        &self,
        workspace_id: Uuid,
    ) -> Result<Vec<String>, DatabaseError> {
        // Get workspace snapshots
        let snapshots = self
            .context_memory_engine
            .get_workspace_snapshots(&workspace_id.to_string(), 10)
            .await?;

        let mut related_ids = Vec::new();

        for snapshot in snapshots {
            // Find similar snapshots semantically
            let doc_id = format!("snapshot-{}", snapshot.id);
            if let Ok(similar) = self.semantic_search.find_similar(&doc_id, 5, 0.5).await {
                for result in similar {
                    if !related_ids.contains(&result.document.id) {
                        related_ids.push(result.document.id);
                    }
                }
            }
        }

        Ok(related_ids)
    }

    /// Detects recurring workflows in a workspace.
    pub async fn detect_recurring_workflows(
        &self,
        _workspace_id: Uuid,
    ) -> Result<Vec<String>, DatabaseError> {
        // Get recent predictions
        let predictions = self.predictive_engine.get_predictions_summary().await?;

        // Analyze file predictions for patterns
        let mut patterns = Vec::new();

        if predictions.next_files.len() >= 3 {
            patterns.push(format!(
                "Frequently opens: {}",
                predictions
                    .next_files
                    .iter()
                    .take(3)
                    .map(|p| p.file_path.split('/').next_back().unwrap_or(&p.file_path))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        if let Some(workflow) = &predictions.current_workflow {
            patterns.push(format!("Active workflow: {:?}", workflow.workflow_type));
        }

        Ok(patterns)
    }

    /// Identifies similar sessions based on semantic content.
    pub async fn find_similar_sessions(
        &self,
        workspace_id: Uuid,
        limit: usize,
    ) -> Result<Vec<String>, DatabaseError> {
        // Get latest session
        let latest_session = self
            .context_memory_engine
            .get_latest_snapshot(&workspace_id.to_string())
            .await?;

        if let Some(snapshot) = latest_session {
            let doc_id = format!("snapshot-{}", snapshot.id);
            let similar = self
                .semantic_search
                .find_similar(&doc_id, limit, 0.5)
                .await?;

            Ok(similar.into_iter().map(|r| r.document.id).collect())
        } else {
            Ok(vec![])
        }
    }

    /// Explains a recommendation with supporting evidence.
    pub async fn explain_recommendation(
        &self,
        workspace_id: Uuid,
        recommendation_id: String,
    ) -> Result<ExplainablePrediction, DatabaseError> {
        // Get recommendations
        let recommendations = self
            .recommendation_engine
            .generate_recommendations(workspace_id)
            .await?;

        // Find the specific recommendation
        let recommendation = recommendations
            .iter()
            .find(|r| r.id == recommendation_id)
            .ok_or_else(|| DatabaseError::NotFound {
                entity: "Recommendation",
                id: recommendation_id.clone(),
            })?;

        // Build evidence
        let mut evidence = vec![
            Evidence {
                source: "RecommendationEngine".to_string(),
                description: format!("Category: {:?}", recommendation.category),
                confidence: recommendation.confidence as f32,
                data: serde_json::json!({
                    "category": recommendation.category,
                    "priority": recommendation.priority,
                }),
            },
            Evidence {
                source: "ContextMemoryEngine".to_string(),
                description: "Based on recent workspace activity".to_string(),
                confidence: 0.7,
                data: serde_json::Value::Null,
            },
        ];

        // Add semantic evidence if available
        if let Ok(similar) = self
            .semantic_search
            .find_similar(&format!("recommendation-{}", recommendation_id), 3, 0.5)
            .await
        {
            for result in similar {
                evidence.push(Evidence {
                    source: "SemanticSearch".to_string(),
                    description: format!("Similar to: {}", result.document.title),
                    confidence: result.confidence,
                    data: serde_json::json!({
                        "document_id": result.document.id,
                        "score": result.score,
                    }),
                });
            }
        }

        Ok(ExplainablePrediction {
            prediction_type: "recommendation".to_string(),
            value: serde_json::json!(recommendation),
            confidence: recommendation.confidence as f32,
            explanation: format!(
                "{} - Impact: {:.0}%, Effort: {:.0}%",
                recommendation.description,
                recommendation.impact * 100.0,
                recommendation.effort * 100.0
            ),
            supporting_evidence: evidence,
            source_engines: vec![
                "RecommendationEngine".to_string(),
                "ContextMemoryEngine".to_string(),
                "SemanticSearch".to_string(),
            ],
            related_documents: vec![format!("workspace-{}", workspace_id)],
            created_at: Utc::now(),
        })
    }

    /// Infers missing context based on semantic analysis.
    pub async fn infer_missing_context(
        &self,
        workspace_id: Uuid,
    ) -> Result<Vec<String>, DatabaseError> {
        let mut suggestions = Vec::new();

        // Check for incomplete snapshots
        let snapshots = self
            .context_memory_engine
            .get_workspace_snapshots(&workspace_id.to_string(), 5)
            .await?;

        if snapshots.is_empty() {
            suggestions.push("No context snapshots available - consider creating one".to_string());
        }

        // Check for related workspaces
        let related = self
            .context_memory_engine
            .get_related_workspaces(&workspace_id.to_string(), 0.5, 5)
            .await?;

        if related.is_empty() {
            suggestions.push(
                "No related workspaces detected - explore workspace relationships".to_string(),
            );
        }

        // Check for recommendations
        let recommendations = self
            .recommendation_engine
            .generate_recommendations(workspace_id)
            .await?;

        if recommendations.is_empty() {
            suggestions.push("No active recommendations - workspace analysis pending".to_string());
        }

        Ok(suggestions)
    }

    /// Indexes a recommendation for semantic search.
    pub async fn index_recommendation(
        &self,
        workspace_id: Uuid,
        recommendation: &crate::intelligence::recommendation::Recommendation,
    ) -> Result<(), DatabaseError> {
        let request = IndexDocumentRequest {
            id: format!("recommendation-{}", recommendation.id),
            doc_type: SemanticDocumentType::Recommendation,
            workspace_id: Some(workspace_id.to_string()),
            title: recommendation.title.clone(),
            content: recommendation.description.clone(),
            metadata: serde_json::json!({
                "category": recommendation.category,
                "priority": recommendation.priority,
                "confidence": recommendation.confidence,
                "impact": recommendation.impact,
                "effort": recommendation.effort,
            }),
        };

        self.semantic_engine.index_document(request).await?;
        Ok(())
    }

    /// Indexes a context snapshot for semantic search.
    pub async fn index_snapshot(
        &self,
        snapshot: &crate::context_memory::models::ContextSnapshot,
    ) -> Result<(), DatabaseError> {
        let content = format!(
            "Active files: {} | Session: {:?} | Health: {:?}",
            snapshot.active_files.join(", "),
            snapshot.session_summary,
            snapshot.health_score
        );

        let request = IndexDocumentRequest {
            id: format!("snapshot-{}", snapshot.id),
            doc_type: SemanticDocumentType::ContextSnapshot,
            workspace_id: Some(snapshot.workspace_id.clone()),
            title: format!("Context Snapshot {}", snapshot.captured_at),
            content,
            metadata: snapshot.metadata.clone(),
        };

        self.semantic_engine.index_document(request).await?;
        Ok(())
    }
}
