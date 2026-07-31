//! Context Memory & Workspace Intelligence
//!
//! This module provides context snapshots and cross-workspace intelligence.

pub mod engine;
pub mod models;
pub mod repository;

pub use engine::ContextMemoryEngine;
pub use models::{
    ContextSnapshot, CreateSnapshotRequest, KnowledgeQuery, KnowledgeSearchResult,
    RelatedWorkspace, SnapshotType, WorkspaceRelationship, WorkspaceRelationshipType,
};
pub use repository::ContextMemoryRepository;
