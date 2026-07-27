//! Application error types.
//!
//! Phase 2 introduces the first concrete error type, [`DatabaseError`],
//! shared by every layer that can fail for a storage-related reason
//! (connection, migration, query, or constraint violation). As later
//! phases add non-database failure modes (file-watcher I/O, ML inference,
//! etc.) their error types will be added as sibling modules here rather
//! than folded into `DatabaseError`, keeping each error type honest about
//! what can actually produce it.

pub mod database_error;
pub mod watcher_error;

pub use database_error::DatabaseError;
pub use watcher_error::WatcherError;
