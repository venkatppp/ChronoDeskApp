//! Semantic memory repository for SQLite persistence.

use chrono::Utc;
use serde_json;
use sqlx::{Row, SqlitePool};

use crate::errors::DatabaseError;
use crate::semantic::models::{IndexDocumentRequest, SemanticDocument, SemanticDocumentType};

/// Repository for semantic documents.
#[derive(Clone)]
pub struct SemanticRepository {
    pool: SqlitePool,
}

impl SemanticRepository {
    /// Creates a new semantic repository.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initializes the semantic documents table.
    pub async fn initialize(&self) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS semantic_documents (
                id TEXT PRIMARY KEY,
                doc_type TEXT NOT NULL,
                workspace_id TEXT,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT NOT NULL,
                embedding BLOB,
                indexed_at TEXT NOT NULL,
                
                FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_semantic_doc_type 
            ON semantic_documents(doc_type)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_semantic_workspace 
            ON semantic_documents(workspace_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Indexes a semantic document.
    pub async fn index_document(
        &self,
        request: IndexDocumentRequest,
        embedding: Option<Vec<f32>>,
    ) -> Result<SemanticDocument, DatabaseError> {
        let metadata_json = serde_json::to_string(&request.metadata)?;
        let doc_type_str = request.doc_type.as_str();
        let indexed_at = Utc::now().to_rfc3339();

        let embedding_blob = embedding
            .as_ref()
            .map(|e| e.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>());

        sqlx::query(
            r#"
            INSERT INTO semantic_documents (id, doc_type, workspace_id, title, content, metadata, embedding, indexed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                doc_type = excluded.doc_type,
                workspace_id = excluded.workspace_id,
                title = excluded.title,
                content = excluded.content,
                metadata = excluded.metadata,
                embedding = excluded.embedding,
                indexed_at = excluded.indexed_at
            "#,
        )
        .bind(&request.id)
        .bind(doc_type_str)
        .bind(&request.workspace_id)
        .bind(&request.title)
        .bind(&request.content)
        .bind(&metadata_json)
        .bind(embedding_blob)
        .bind(&indexed_at)
        .execute(&self.pool)
        .await?;

        Ok(SemanticDocument {
            id: request.id,
            doc_type: request.doc_type,
            workspace_id: request.workspace_id,
            title: request.title,
            content: request.content,
            metadata: request.metadata,
            embedding,
            indexed_at: chrono::DateTime::parse_from_rfc3339(&indexed_at)
                .map_err(|e| DatabaseError::InvalidInput(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&Utc),
        })
    }

    /// Gets a semantic document by ID.
    pub async fn get_document(&self, id: &str) -> Result<Option<SemanticDocument>, DatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT id, doc_type, workspace_id, title, content, metadata, embedding, indexed_at
            FROM semantic_documents
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let doc_type_str: String = row.get("doc_type");
            let doc_type = self.parse_doc_type(&doc_type_str)?;

            let metadata_json: String = row.get("metadata");
            let metadata: serde_json::Value = serde_json::from_str(&metadata_json)?;

            let embedding_blob: Option<Vec<u8>> = row.get("embedding");
            let embedding = embedding_blob.map(|blob| {
                blob.chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect()
            });

            let indexed_at_str: String = row.get("indexed_at");
            let indexed_at = chrono::DateTime::parse_from_rfc3339(&indexed_at_str)
                .map_err(|e| DatabaseError::InvalidInput(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&Utc);

            Ok(Some(SemanticDocument {
                id: row.get("id"),
                doc_type,
                workspace_id: row.get("workspace_id"),
                title: row.get("title"),
                content: row.get("content"),
                metadata,
                embedding,
                indexed_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Searches documents by type and workspace.
    pub async fn search_by_type(
        &self,
        doc_type: &SemanticDocumentType,
        workspace_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SemanticDocument>, DatabaseError> {
        let doc_type_str = doc_type.as_str();

        let query = if workspace_id.is_some() {
            r#"
            SELECT id, doc_type, workspace_id, title, content, metadata, embedding, indexed_at
            FROM semantic_documents
            WHERE doc_type = ? AND workspace_id = ?
            ORDER BY indexed_at DESC
            LIMIT ?
            "#
        } else {
            r#"
            SELECT id, doc_type, workspace_id, title, content, metadata, embedding, indexed_at
            FROM semantic_documents
            WHERE doc_type = ?
            ORDER BY indexed_at DESC
            LIMIT ?
            "#
        };

        let rows = if let Some(ws_id) = workspace_id {
            sqlx::query(query)
                .bind(doc_type_str)
                .bind(ws_id)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(query)
                .bind(doc_type_str)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await?
        };

        self.rows_to_documents(rows).await
    }

    /// Deletes a document by ID.
    pub async fn delete_document(&self, id: &str) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM semantic_documents WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Helper to parse document type from string.
    fn parse_doc_type(&self, s: &str) -> Result<SemanticDocumentType, DatabaseError> {
        match s {
            "workspace" => Ok(SemanticDocumentType::Workspace),
            "file" => Ok(SemanticDocumentType::File),
            "session" => Ok(SemanticDocumentType::Session),
            "context_snapshot" => Ok(SemanticDocumentType::ContextSnapshot),
            "graph_node" => Ok(SemanticDocumentType::GraphNode),
            "recommendation" => Ok(SemanticDocumentType::Recommendation),
            "timeline_event" => Ok(SemanticDocumentType::TimelineEvent),
            "analytics_summary" => Ok(SemanticDocumentType::AnalyticsSummary),
            _ => Err(DatabaseError::InvalidInput(format!(
                "Unknown document type: {}",
                s
            ))),
        }
    }

    /// Helper to convert rows to documents.
    async fn rows_to_documents(
        &self,
        rows: Vec<sqlx::sqlite::SqliteRow>,
    ) -> Result<Vec<SemanticDocument>, DatabaseError> {
        let mut documents = Vec::new();

        for row in rows {
            let doc_type_str: String = row.get("doc_type");
            let doc_type = self.parse_doc_type(&doc_type_str)?;

            let metadata_json: String = row.get("metadata");
            let metadata: serde_json::Value = serde_json::from_str(&metadata_json)?;

            let embedding_blob: Option<Vec<u8>> = row.get("embedding");
            let embedding = embedding_blob.map(|blob| {
                blob.chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect()
            });

            let indexed_at_str: String = row.get("indexed_at");
            let indexed_at = chrono::DateTime::parse_from_rfc3339(&indexed_at_str)
                .map_err(|e| DatabaseError::InvalidInput(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&Utc);

            documents.push(SemanticDocument {
                id: row.get("id"),
                doc_type,
                workspace_id: row.get("workspace_id"),
                title: row.get("title"),
                content: row.get("content"),
                metadata,
                embedding,
                indexed_at,
            });
        }

        Ok(documents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    async fn setup() -> (SemanticRepository, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test.db");
        let db = Database::initialize_at(&db_path).await.unwrap();
        let repo = SemanticRepository::new(db.pool().clone());
        repo.initialize().await.unwrap();
        (repo, tmp)
    }

    #[tokio::test]
    async fn index_and_get_document() {
        let (repo, _tmp) = setup().await;

        let request = IndexDocumentRequest {
            id: "test-1".to_string(),
            doc_type: SemanticDocumentType::Workspace,
            workspace_id: None,
            title: "Test Workspace".to_string(),
            content: "This is a test workspace".to_string(),
            metadata: serde_json::json!({"test": true}),
        };

        let doc = repo.index_document(request, None).await.unwrap();
        assert_eq!(doc.id, "test-1");

        let retrieved = repo.get_document("test-1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Workspace");
    }

    #[tokio::test]
    async fn search_by_type_filters_correctly() {
        let (repo, _tmp) = setup().await;

        repo.index_document(
            IndexDocumentRequest {
                id: "ws-1".to_string(),
                doc_type: SemanticDocumentType::Workspace,
                workspace_id: None,
                title: "Workspace 1".to_string(),
                content: "Content 1".to_string(),
                metadata: serde_json::Value::Null,
            },
            None,
        )
        .await
        .unwrap();

        repo.index_document(
            IndexDocumentRequest {
                id: "file-1".to_string(),
                doc_type: SemanticDocumentType::File,
                workspace_id: None,
                title: "File 1".to_string(),
                content: "Content 2".to_string(),
                metadata: serde_json::Value::Null,
            },
            None,
        )
        .await
        .unwrap();

        let results = repo
            .search_by_type(&SemanticDocumentType::Workspace, None, 10)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "ws-1");
    }
}
