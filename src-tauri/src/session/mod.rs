//! Session Intelligence Module
//!
//! Reconstructs work sessions from timeline events and provides context
//! intelligence for the Smart Resume feature. Sessions are derived from
//! timeline events, not stored as canonical data — timeline events remain
//! the single source of truth.
//!
//! ## Architecture
//!
//! ```text
//! timeline_events (source of truth)
//!     │
//!     ▼
//! detector::detect_sessions()    (reconstruct sessions from events)
//!     │
//!     ▼
//! SessionEngine                   (session operations + scoring)
//!     │
//!     ▼
//! ContextService                  (high-level intelligence API)
//!     │
//!     ▼
//! commands::session               (IPC boundary)
//! ```

pub mod detector;
pub mod engine;
pub mod language_detection;
pub mod scoring;
pub mod types;

pub use engine::SessionEngine;
pub use types::{Session, SessionContext, SessionSummary};
