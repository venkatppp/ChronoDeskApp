//! Autonomous Agent Runtime (RC-5 M6).
//!
//! A thin orchestration layer over the existing [`Planner`] and
//! [`ExecutionEngine`]: it runs autonomous *sessions* — a
//! reason–act–observe loop that plans, executes one plan run through the
//! engine, observes the outcome, and replans (feedback-aware) until the
//! budget is spent or the goal is reached.
//!
//! Responsibility split stays frozen from the RC-5 charter:
//! - `Planner` builds/revises the DAG.
//! - `ExecutionEngine` schedules/lifecycles one plan run, persists, streams
//!   `execution:progress`.
//! - `ToolExecutor` is the only execution path for a single tool.
//! - This runtime - owns the *session*: budgets, retry/timeout policies,
//!   approval checkpoints (human-in-the-loop), reasoning event streaming,
//!   and autonomous cancellation. It never schedules a step itself and
//!   never calls a tool directly.

pub mod models;
pub mod runtime;

pub use models::{
    ApprovalMode, ApprovalPolicy, ApprovalRequest, AutonomousSessionProgress, AutonomousStatus,
    ExecutionBudget, ExecutionPolicy, ReasoningEvent, ReasoningPhase, RetryPolicy, TimeoutPolicy,
};
pub use runtime::{AutonomousRuntime, AutonomousRuntimeError};
