//! Semantic Search Engine
//!
//! Provides natural-language search across all indexed documents.

use crate::errors::DatabaseError;
use crate::semantic::engine::SemanticMemoryEngine;
use crate::semantic::models::{SemanticDocumentType, SemanticSearchRequest, SemanticSearchResult};
use crate::semantic::repository::SemanticRepository;

/// Semantic search engine for natural-language queries.
#[derive(Clone)]
pub struct SemanticSearchEngine {
    memory_engine: SemanticMemoryEngine,
    repository: SemanticRepository,
}

impl SemanticSearchEngine {
    /// Creates a new semantic search engine.
    pub fn new(memory_engine: SemanticMemoryEngine, repository: SemanticRepository) -> Self {
        Self {
            memory_engine,
            repository,
        }
    }

    /// Searches semantic memory with a natural-language query.
    pub async fn search(
        &self,
        request: SemanticSearchRequest,
    ) -> Result<Vec<SemanticSearchResult>, DatabaseError> {
        // Generate query embedding
        let query_embedding = self
            .memory_engine
            .embedding_provider
            .embed(&request.query)
            .await?;

        // Get candidate documents
        let doc_types = request.doc_types.unwrap_or_else(|| {
            vec![
                SemanticDocumentType::Workspace,
                SemanticDocumentType::File,
                SemanticDocumentType::Session,
                SemanticDocumentType::ContextSnapshot,
                SemanticDocumentType::GraphNode,
                SemanticDocumentType::Recommendation,
                SemanticDocumentType::TimelineEvent,
                SemanticDocumentType::AnalyticsSummary,
            ]
        });

        let mut candidates = Vec::new();
        for doc_type in doc_types {
            let docs = self
                .repository
                .search_by_type(&doc_type, request.workspace_id.as_deref(), 100)
                .await?;
            candidates.extend(docs);
        }

        // Compute similarities and rank
        let mut results: Vec<SemanticSearchResult> = candidates
            .into_iter()
            .filter_map(|doc| {
                if let Some(ref embedding) = doc.embedding {
                    let similarity = self
                        .memory_engine
                        .cosine_similarity(&query_embedding, embedding);

                    if similarity >= request.min_confidence {
                        Some(SemanticSearchResult {
                            document: doc,
                            score: similarity,
                            confidence: similarity,
                            explanation: Some(format!(
                                "Semantic similarity: {:.2}%",
                                similarity * 100.0
                            )),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        results.truncate(request.limit);

        Ok(results)
    }

    /// Finds similar documents to a given document ID.
    pub async fn find_similar(
        &self,
        document_id: &str,
        limit: usize,
        min_confidence: f32,
    ) -> Result<Vec<SemanticSearchResult>, DatabaseError> {
        let source_doc = self
            .memory_engine
            .get_document(document_id)
            .await?
            .ok_or_else(|| DatabaseError::NotFound {
                entity: "SemanticDocument",
                id: document_id.to_string(),
            })?;

        let source_embedding = source_doc
            .embedding
            .ok_or_else(|| DatabaseError::InvalidInput("Document has no embedding".to_string()))?;

        // Get all documents of the same type
        let candidates = self
            .repository
            .search_by_type(
                &source_doc.doc_type,
                source_doc.workspace_id.as_deref(),
                1000,
            )
            .await?;

        // Compute similarities
        let mut results: Vec<SemanticSearchResult> = candidates
            .into_iter()
            .filter(|doc| doc.id != document_id) // Exclude source document
            .filter_map(|doc| {
                if let Some(ref embedding) = doc.embedding {
                    let similarity = self
                        .memory_engine
                        .cosine_similarity(&source_embedding, embedding);

                    if similarity >= min_confidence {
                        Some(SemanticSearchResult {
                            document: doc,
                            score: similarity,
                            confidence: similarity,
                            explanation: Some(format!("Similarity: {:.2}%", similarity * 100.0)),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        results.truncate(limit);

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::semantic::embeddings::LocalEmbeddingProvider;
    use crate::semantic::models::IndexDocumentRequest;
    use std::sync::Arc;

    async fn setup() -> (SemanticSearchEngine, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test.db");
        let db = Database::initialize_at(&db_path).await.unwrap();
        let repo = SemanticRepository::new(db.pool().clone());
        repo.initialize().await.unwrap();

        let provider = Arc::new(LocalEmbeddingProvider::default());
        let memory_engine = SemanticMemoryEngine::new(repo.clone(), provider);
        let search_engine = SemanticSearchEngine::new(memory_engine.clone(), repo);

        // Index some test documents
        memory_engine
            .index_document(IndexDocumentRequest {
                id: "doc-1".to_string(),
                doc_type: SemanticDocumentType::Workspace,
                workspace_id: None,
                title: "Authentication Bug".to_string(),
                content: "Fixed authentication issue with JWT tokens".to_string(),
                metadata: serde_json::Value::Null,
            })
            .await
            .unwrap();

        memory_engine
            .index_document(IndexDocumentRequest {
                id: "doc-2".to_string(),
                doc_type: SemanticDocumentType::Session,
                workspace_id: None,
                title: "Redis Implementation".to_string(),
                content: "Implemented Redis caching for session management".to_string(),
                metadata: serde_json::Value::Null,
            })
            .await
            .unwrap();

        (search_engine, tmp)
    }

    #[tokio::test]
    async fn search_returns_relevant_results() {
        let (engine, _tmp) = setup().await;

        let request = SemanticSearchRequest {
            query: "authentication".to_string(),
            ..Default::default()
        };

        let results = engine.search(request).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn search_respects_confidence_threshold() {
        let (engine, _tmp) = setup().await;

        let request = SemanticSearchRequest {
            query: "test".to_string(),
            min_confidence: 0.99,
            ..Default::default()
        };

        let results = engine.search(request).await.unwrap();
        for result in results {
            assert!(result.confidence >= 0.99);
        }
    }

    #[tokio::test]
    async fn find_similar_excludes_source_document() {
        let (engine, _tmp) = setup().await;

        let results = engine.find_similar("doc-1", 10, 0.0).await.unwrap();

        for result in &results {
            assert_ne!(result.document.id, "doc-1");
        }
    }
}
