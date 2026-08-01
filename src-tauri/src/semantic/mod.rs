//! Semantic Intelligence Layer
//!
//! Provides semantic memory, embedding-based search, and AI reasoning
//! capabilities across all ChronoDesk data types.

pub mod embeddings;
pub mod engine;
pub mod models;
pub mod reasoning;
pub mod repository;
pub mod search;

pub use embeddings::{EmbeddingProvider, LocalEmbeddingProvider};
pub use engine::SemanticMemoryEngine;
pub use models::{
    ExplainablePrediction, SemanticDocument, SemanticDocumentType, SemanticSearchResult,
};
pub use reasoning::ContextReasoningEngine;
pub use repository::SemanticRepository;
pub use search::SemanticSearchEngine;
