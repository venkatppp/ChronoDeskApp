//! Semantic Memory Engine
//!
//! Manages semantic indexing and retrieval across all ChronoDesk data.

use std::sync::Arc;

use crate::errors::DatabaseError;
use crate::semantic::embeddings::EmbeddingProvider;
use crate::semantic::models::{IndexDocumentRequest, SemanticDocument};
use crate::semantic::repository::SemanticRepository;

/// Semantic memory engine for indexing and retrieval.
#[derive(Clone)]
pub struct SemanticMemoryEngine {
    repository: SemanticRepository,
    pub(crate) embedding_provider: Arc<dyn EmbeddingProvider>,
}

impl SemanticMemoryEngine {
    /// Creates a new semantic memory engine.
    pub fn new(
        repository: SemanticRepository,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            repository,
            embedding_provider,
        }
    }

    /// Indexes a document with automatic embedding generation.
    pub async fn index_document(
        &self,
        request: IndexDocumentRequest,
    ) -> Result<SemanticDocument, DatabaseError> {
        // Generate embedding for the content
        let combined_text = format!("{} {}", request.title, request.content);
        let embedding = self.embedding_provider.embed(&combined_text).await?;

        // Store in repository
        self.repository
            .index_document(request, Some(embedding))
            .await
    }

    /// Gets a document by ID.
    pub async fn get_document(&self, id: &str) -> Result<Option<SemanticDocument>, DatabaseError> {
        self.repository.get_document(id).await
    }

    /// Deletes a document.
    pub async fn delete_document(&self, id: &str) -> Result<(), DatabaseError> {
        self.repository.delete_document(id).await
    }

    /// Computes cosine similarity between two embeddings.
    pub fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            return 0.0;
        }

        dot_product / (magnitude_a * magnitude_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::semantic::embeddings::LocalEmbeddingProvider;
    use crate::semantic::models::SemanticDocumentType;

    async fn setup() -> (SemanticMemoryEngine, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test.db");
        let db = Database::initialize_at(&db_path).await.unwrap();
        let repo = SemanticRepository::new(db.pool().clone());
        repo.initialize().await.unwrap();

        let provider = Arc::new(LocalEmbeddingProvider::default());
        let engine = SemanticMemoryEngine::new(repo, provider);

        (engine, tmp)
    }

    #[tokio::test]
    async fn index_document_generates_embedding() {
        let (engine, _tmp) = setup().await;

        let request = IndexDocumentRequest {
            id: "test-1".to_string(),
            doc_type: SemanticDocumentType::Workspace,
            workspace_id: None,
            title: "Test".to_string(),
            content: "Content".to_string(),
            metadata: serde_json::Value::Null,
        };

        let doc = engine.index_document(request).await.unwrap();
        assert!(doc.embedding.is_some());
        assert_eq!(doc.embedding.unwrap().len(), 384);
    }

    #[tokio::test]
    async fn cosine_similarity_computes_correctly() {
        let (engine, _tmp) = setup().await;

        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let similarity = engine.cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 0.001);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        let similarity = engine.cosine_similarity(&c, &d);
        assert!((similarity - 0.0).abs() < 0.001);
    }
}
