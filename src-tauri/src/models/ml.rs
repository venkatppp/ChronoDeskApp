//! ML metadata models (Phase 5).
//!
//! These models represent ML-derived insights about files: classification
//! labels, embedding references, and confidence scores. The actual embedding
//! vectors are stored in a separate `embeddings` table; `embedding_id` here
//! is just a reference.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::errors::DatabaseError;

/// File classification category (Phase 5 ML Layer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileClassification {
    Code,
    Document,
    Data,
    Media,
    Configuration,
    Unknown,
}

impl FileClassification {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileClassification::Code => "code",
            FileClassification::Document => "document",
            FileClassification::Data => "data",
            FileClassification::Media => "media",
            FileClassification::Configuration => "configuration",
            FileClassification::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for FileClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for FileClassification {
    type Err = DatabaseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "code" => Ok(FileClassification::Code),
            "document" => Ok(FileClassification::Document),
            "data" => Ok(FileClassification::Data),
            "media" => Ok(FileClassification::Media),
            "configuration" => Ok(FileClassification::Configuration),
            "unknown" => Ok(FileClassification::Unknown),
            other => Err(DatabaseError::InvalidInput(format!(
                "unknown file classification '{other}'"
            ))),
        }
    }
}

/// ML metadata for a file (blueprint §6, Phase 5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MLMetadata {
    pub id: Uuid,
    pub file_id: Uuid,
    pub model_version: String,
    /// Reference to an `embeddings` table row, not the vector itself.
    pub embedding_id: Option<String>,
    pub classification: Option<FileClassification>,
    pub confidence: Option<f32>,
    pub created_at: DateTime<Utc>,
}

/// Raw shape of an `ml_metadata` row; classification is decoded as `String`
/// first (SQLite has no enum type), then converted via `FromStr`.
#[derive(Debug, FromRow)]
pub(crate) struct MLMetadataRow {
    pub id: Uuid,
    pub file_id: Uuid,
    pub model_version: String,
    pub embedding_id: Option<String>,
    pub classification: Option<String>,
    pub confidence: Option<f32>,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<MLMetadataRow> for MLMetadata {
    type Error = DatabaseError;

    fn try_from(row: MLMetadataRow) -> Result<Self, Self::Error> {
        let classification = match row.classification {
            Some(ref s) => Some(s.parse()?),
            None => None,
        };

        Ok(MLMetadata {
            id: row.id,
            file_id: row.file_id,
            model_version: row.model_version,
            embedding_id: row.embedding_id,
            classification,
            confidence: row.confidence,
            created_at: row.created_at,
        })
    }
}

/// Input for creating new ML metadata.
#[derive(Debug, Clone)]
pub struct NewMLMetadata {
    pub file_id: Uuid,
    pub model_version: String,
    pub embedding_id: Option<String>,
    pub classification: Option<FileClassification>,
    pub confidence: Option<f32>,
}

/// An embedding vector stored in the `embeddings` table (Phase 5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Embedding {
    pub id: String,
    /// The actual vector, stored as BLOB in SQLite, serialized here.
    pub vector: Vec<f32>,
    pub dimensions: i32,
    pub model_version: String,
    pub created_at: DateTime<Utc>,
}

/// Raw shape of an `embeddings` row.
#[derive(Debug, FromRow)]
pub(crate) struct EmbeddingRow {
    pub id: String,
    pub vector: Vec<u8>,
    pub dimensions: i32,
    pub model_version: String,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<EmbeddingRow> for Embedding {
    type Error = DatabaseError;

    fn try_from(row: EmbeddingRow) -> Result<Self, Self::Error> {
        // Deserialize BLOB back to Vec<f32>. Each f32 is 4 bytes (little-endian).
        if row.vector.len() % 4 != 0 {
            return Err(DatabaseError::InvalidInput(
                "embedding vector BLOB size is not a multiple of 4".to_string(),
            ));
        }

        let vector: Vec<f32> = row
            .vector
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        if vector.len() != row.dimensions as usize {
            return Err(DatabaseError::InvalidInput(format!(
                "embedding vector length {} does not match dimensions {}",
                vector.len(),
                row.dimensions
            )));
        }

        Ok(Embedding {
            id: row.id,
            vector,
            dimensions: row.dimensions,
            model_version: row.model_version,
            created_at: row.created_at,
        })
    }
}

/// Input for creating a new embedding.
#[derive(Debug, Clone)]
pub struct NewEmbedding {
    pub id: String,
    pub vector: Vec<f32>,
    pub model_version: String,
}
