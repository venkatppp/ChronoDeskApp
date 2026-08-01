use serde::Serialize;

/// Every failure mode the storage layer can produce.
///
/// This is the error type returned by [`crate::database`],
/// [`crate::repositories`], [`crate::services::workspace_service`], and —
/// via [`serde::Serialize`] — directly by Tauri commands, so a failure
/// deep in a repository surfaces to the frontend as a structured,
/// human-readable message instead of a panic or an opaque string.
///
/// Variants are deliberately specific (`NotFound`, `Constraint`,
/// `Migration`, ...) rather than one catch-all `Sqlx(sqlx::Error)`, so
/// callers — and the UI — can react differently to "this workspace
/// doesn't exist" versus "the database file is locked".
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// The database connection pool could not be created, or the
    /// underlying SQLite file could not be opened/created.
    #[error("failed to connect to the database: {0}")]
    Connection(#[source] sqlx::Error),

    /// A migration failed to apply. This is treated as fatal at startup —
    /// ChronoDesk refuses to run against a database in an unknown schema
    /// state rather than silently operating on partial/incorrect tables.
    #[error("database migration failed: {0}")]
    Migration(#[source] sqlx::migrate::MigrateError),

    /// A query executed successfully as SQL but failed for a
    /// database-level reason (I/O, corruption, disk full, etc.).
    #[error("database query failed: {0}")]
    Query(#[source] sqlx::Error),

    /// A query that expected exactly one row (e.g. "get workspace by id")
    /// found none.
    #[error("{entity} with id '{id}' was not found")]
    NotFound { entity: &'static str, id: String },

    /// A write violated a schema constraint (`CHECK`, `UNIQUE`, foreign
    /// key). Kept distinct from `Query` so the service layer can turn
    /// this into a specific, actionable user-facing message instead of a
    /// generic "database error".
    #[error("constraint violation: {0}")]
    Constraint(String),

    /// The caller supplied a value that is syntactically valid but
    /// semantically invalid for the operation (e.g. an empty workspace
    /// name, or a health score outside 0–100).
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Failed to resolve the application's data directory (Tauri's path
    /// resolver returned an error, or the directory could not be created).
    #[error("failed to resolve application data directory: {0}")]
    AppDataDir(String),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O error (file operations, network, etc.).
    #[error("I/O error: {0}")]
    IoError(String),
}

impl DatabaseError {
    /// Builds a [`DatabaseError::NotFound`] for the given entity/id pair.
    /// Centralized so the `entity` label stays consistent across
    /// repositories (`"workspace"`, `"file"`, `"timeline_event"`, ...).
    pub fn not_found(entity: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity,
            id: id.into(),
        }
    }
}

/// Classifies a raw [`sqlx::Error`] into the appropriate [`DatabaseError`]
/// variant. SQLite reports constraint violations as
/// `sqlx::Error::Database` with a database-specific error code, so this
/// distinguishes "row not found" and "constraint failed" from generic
/// query failures rather than collapsing everything into `Query`.
impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => DatabaseError::NotFound {
                entity: "row",
                id: "unknown".to_string(),
            },
            sqlx::Error::Database(db_err) => {
                if db_err.is_unique_violation() || db_err.is_foreign_key_violation() {
                    DatabaseError::Constraint(db_err.message().to_string())
                } else {
                    DatabaseError::Query(err)
                }
            }
            _ => DatabaseError::Query(err),
        }
    }
}

impl From<sqlx::migrate::MigrateError> for DatabaseError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        DatabaseError::Migration(err)
    }
}

/// Manual [`Serialize`] implementation (rather than `#[derive(Serialize)]`,
/// which can't be derived here since `sqlx::Error` isn't `Serialize`)
/// so this error type can be returned directly as the `E` in a Tauri
/// command's `Result<T, E>` and cross the IPC boundary as a plain string.
impl Serialize for DatabaseError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
