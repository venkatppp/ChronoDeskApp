//! Analytics Module
//!
//! Provides context intelligence through data aggregation and analysis.
//! Built on top of ContextService and SessionEngine to deliver daily/weekly/monthly
//! summaries, activity trends, workspace insights, and productivity analytics.
//!
//! ## Architecture
//!
//! ```text
//! AnalyticsEngine (high-level analytics API)
//!     │
//!     ▼
//! AnalyticsService (data transformation & business logic)
//!     │
//!     ▼
//! AnalyticsRepository (efficient SQL aggregations)
//!     │
//!     ▼
//! Database (timeline_events, workspaces, files, sessions via ContextService)
//! ```
//!
//! Analytics consumes existing services (ContextService, SessionEngine) rather
//! than duplicating session logic.

pub mod engine;
pub mod models;
pub mod repository;
pub mod service;

pub use engine::AnalyticsEngine;
pub use models::{
    ActivitySummary, DailyBriefing, DailySummary, LanguageUsage, MonthlySummary, TimeRange,
    TrendIndicator, WeeklySummary, WorkspaceInsight,
};
