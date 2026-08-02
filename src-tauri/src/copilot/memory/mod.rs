//! Execution Memory & Learning (RC-6 M1) — now a complete long-term
//! memory system (RC-6 M4): a durable store of plan executions, planner
//! reports, and autonomous sessions; semantic retrieval over remembered
//! goals/plans; a learning engine that ranks history, recommends
//! successful workflows, and flags failed strategies; and full lifecycle
//! management — retention policies, automatic cleanup, compression,
//! versioning + lineage, import/export, snapshots, and storage stats.
//!
//! Ownership: the memory store is *read/write-only*. It never schedules
//! executions (that is `ExecutionEngine`), never plans (that is
//! `Planner`), and never drives a session (that is `AutonomousRuntime`).
//! Those components consult the store through [`MemoryEngine`] and record
//! into it at their own lifecycle boundaries.

pub mod cleanup_worker;
pub mod engine;
pub mod learning;
pub mod lifecycle;
pub mod lifecycle_repository;
pub mod models;
pub mod repository;
pub mod retrieval;
pub mod vector;

pub use cleanup_worker::MemoryCleanupWorker;
pub use engine::MemoryEngine;
pub use learning::{
    DuplicateGroup, FailurePattern, LearningHealth, MemoryAgingSummary, MergeResult, WorkflowFamily,
};
pub use lifecycle_repository::LifecycleRepository;
pub use models::{
    AvoidedStrategy, CleanupReport, CompressionResult, ExecutionMemoryRecord, ImportResult,
    LearnedWorkflow, LineageNode, LineageRelation, MemoryAcceptance, MemoryAcceptanceEntry,
    MemoryExport, MemoryHit, MemoryKind, MemoryLineage, MemoryOutcome, MemoryRecommendation,
    MemorySearchRequest, MemorySnapshot, MemoryStats, MemoryStatus, MemoryStorageStats,
    RestoreResult, RetentionPolicy,
};
pub use repository::MemoryRepository;
pub use vector::{IndexResult, LocalVectorProvider, MemoryVectorSystem, VectorIndexStatus};
