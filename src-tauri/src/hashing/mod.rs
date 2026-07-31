//! Hashing infrastructure for duplicate detection (Phase 5 Stage 2).
//!
//! Provides SHA-256 content hashing with buffered streaming I/O, ensuring
//! memory-efficient operation even for very large files. All file access
//! errors (locked, deleted, permission denied) are gracefully handled and
//! returned as typed errors rather than panicking.

mod service;

pub use service::{HashingError, HashingService};
