//! Duplicate detection engine (Phase 5 Stage 2).
//!
//! Provides incremental, resumable duplicate file detection using SHA-256
//! content hashing. Designed for background operation without blocking the UI.

mod engine;

pub use engine::{DuplicateDetectionEngine, DuplicateGroup, ScanProgress};
