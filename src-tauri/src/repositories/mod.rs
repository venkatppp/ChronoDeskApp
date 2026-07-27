//! Repository Pattern: the only layer allowed to run SQL.
//!
//! Each repository owns one aggregate's persistence (create/read/update/
//! delete plus a handful of aggregate-specific queries) and returns
//! strongly typed [`crate::models`] values or a [`crate::errors::DatabaseError`].
//! Services and commands depend on these types, never on `sqlx::SqlitePool`
//! directly — that keeps every SQL string in exactly one place per table
//! and makes each repository independently testable against a temporary
//! database (see the `#[cfg(test)]` modules in each file).

pub mod file_repository;
pub mod graph_repository;
pub mod search_repository;
pub mod settings_repository;
pub mod timeline_repository;
pub mod workspace_repository;

pub use file_repository::FileRepository;
pub use graph_repository::GraphRepository;
pub use search_repository::SearchRepository;
pub use settings_repository::SettingsRepository;
pub use timeline_repository::TimelineRepository;
pub use workspace_repository::WorkspaceRepository;
