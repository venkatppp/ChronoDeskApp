use std::path::Path;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;

use crate::errors::DatabaseError;

/// Maximum number of pooled connections.
///
/// SQLite allows one writer at a time regardless of pool size (WAL mode
/// lets readers proceed concurrently with a writer, but writers still
/// serialize), so a large pool wouldn't buy write throughput — this size
/// is chosen to give concurrent *reads* (e.g. the dashboard querying
/// workspaces while the watcher inserts a timeline event) enough headroom
/// without holding more OS file handles than a desktop app needs.
const MAX_POOL_CONNECTIONS: u32 = 8;

/// Acquire-connection timeout. Fails fast with a clear error rather than
/// hanging the UI indefinitely if the database file is somehow locked by
/// another process (e.g. a second app instance).
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(10);

/// Opens (creating if necessary) the SQLite database at `db_path` and
/// returns a ready-to-use connection pool.
///
/// Every connection in the pool is configured, via
/// [`SqliteConnectOptions`], with:
/// - **WAL journal mode** — readers never block writers and vice versa,
///   which matters here because the dashboard's read queries and the
///   (Phase 3) file watcher's event inserts happen concurrently.
/// - **Foreign keys ON** — SQLite disables foreign-key enforcement by
///   default *per connection*; it cannot be turned on once in the schema,
///   so it's set here rather than in a migration.
/// - **NORMAL synchronous mode** — the standard, safe pairing with WAL;
///   full `FULL` synchronous mode is unnecessary overhead for a local,
///   single-user desktop database.
///
/// # Errors
/// Returns [`DatabaseError::Connection`] if the database file cannot be
/// created/opened (e.g. the parent directory doesn't exist or isn't
/// writable).
pub async fn create_pool(db_path: &Path) -> Result<SqlitePool, DatabaseError> {
    let connect_options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(MAX_POOL_CONNECTIONS)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .connect_with(connect_options)
        .await
        .map_err(DatabaseError::Connection)?;

    tracing::info!(path = %db_path.display(), "SQLite connection pool established (WAL mode, foreign keys ON)");

    Ok(pool)
}
