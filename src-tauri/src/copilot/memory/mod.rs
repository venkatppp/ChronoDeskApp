//! Execution Memory & Learning (RC-6 M1).
//!
//! Lets ChronoDesk learn from previous executions: a durable store of
//! plan executions, planner reports, and autonomous sessions; semantic
//! retrieval over remembered goals/plans; and a learning engine that
//! ranks history, recommends successful workflows, and flags failed
//! strategies to avoid.
//!
//! Ownership: the memory store is *read/write-only*. It never schedules
//! executions (that is `ExecutionEngine`), never plans (that is
//! `Planner`), and never drives a session (that is `AutonomousRuntime`).
//! Those components consult the store through [`MemoryEngine`] and record
//! into it at their own lifecycle boundaries.

pub mod engine;
pub mod learning;
pub mod models;
pub mod repository;
pub mod retrieval;
pub mod vector;

pub use engine::MemoryEngine;
pub use learning::{
    DuplicateGroup, FailurePattern, LearningHealth, MemoryAgingSummary, MergeResult, WorkflowFamily,
};
pub use models::{
    AvoidedStrategy, ExecutionMemoryRecord, LearnedWorkflow, MemoryAcceptance, MemoryHit,
    MemoryKind, MemoryOutcome, MemoryRecommendation, MemorySearchRequest, MemoryStats,
    MemoryStatus,
};
pub use repository::MemoryRepository;
pub use vector::{IndexResult, LocalVectorProvider, MemoryVectorSystem, VectorIndexStatus};
