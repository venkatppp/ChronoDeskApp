//! Adaptive Learning (RC-6 M3) — turns raw execution memory into ranked,
//! *explained* knowledge: which workflows to recommend (with a confidence
//! score and the reasons behind it), which strategies to avoid, which
//! memories to fade out, which duplicates to merge, and how healthy the
//! learning system is.
//!
//! Layering (strict, no duplicated logic):
//!
//! | Concern | Module | Owns |
//! |---------|--------|------|
//! | Blend weights | `weights` | adaptive recommendation weights (learned from history) |
//! | Confidence | `confidence` | `confidence_score` + per-factor explanations |
//! | Aging | `aging` | decay / freshness / archival weighting + summary |
//! | Duplicates | `duplicates` | identical-memory detection + merge plan |
//! | Clustering | `clustering` | reusable workflow families |
//! | Failures | `failures` | repeated failures / unstable workflows / low-confidence plans |
//! | Health | `stats` | learning health payload for the dashboard |
//! | Core | `core` | relevance thresholds, ranking, workflow aggregation, stats |
//!
//! Everything here is pure logic over [`ExecutionMemoryRecord`]s so every
//! rule is unit-testable without a database. The [`MemoryEngine`] facade
//! is the only entry point the planner / runtime / IPC talk to.

pub mod aging;
pub mod clustering;
pub mod confidence;
pub mod core;
pub mod duplicates;
pub mod failures;
pub mod stats;
pub mod weights;

pub use aging::{aging_summary, archival_weight, freshness, MemoryAgingSummary};
pub use clustering::{workflow_families, WorkflowFamily};
pub use confidence::{confidence_score, ConfidenceResult};
pub use core::{
    avoid_strategies, compute_stats, learned_score, learned_workflows, rank_historical,
    RECOMMENDATION_THRESHOLD, RELEVANCE_THRESHOLD,
};
pub use duplicates::{duplicate_groups, merge_plan, DuplicateGroup, MergeResult};
pub use failures::{
    failure_patterns, failure_patterns_for_goal, FailurePattern, FailurePatternType,
};
pub use stats::{
    learning_health, LearningHealth, MemoryUtilization, SuccessTrend, WorkflowQuality,
};
pub use weights::{default_weights, learn_weights, LearningWeights};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
