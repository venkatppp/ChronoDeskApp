use sqlx::SqlitePool;

use crate::errors::DatabaseError;

/// Applies every pending migration under `src-tauri/migrations/` to
/// `pool`, in filename order.
///
/// [`sqlx::migrate!`] embeds the migration files into the compiled binary
/// at build time (path is resolved relative to `CARGO_MANIFEST_DIR`, i.e.
/// `src-tauri/`), so a shipped ChronoDesk build carries its own schema
/// history and never reads `.sql` files off disk at runtime. sqlx tracks
/// which migrations have already run in a `_sqlx_migrations` bookkeeping
/// table it manages itself, so calling this on every startup is
/// idempotent — already-applied migrations are skipped.
///
/// # Errors
/// Returns [`DatabaseError::Migration`] if a migration fails to apply
/// (e.g. a syntax error, or the on-disk schema was hand-modified and no
/// longer matches sqlx's applied-migration checksums). This is treated as
/// fatal — see [`crate::database::initialize`].
pub async fn run(pool: &SqlitePool) -> Result<(), DatabaseError> {
    tracing::info!("running database migrations");

    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "database migration failed");
            DatabaseError::from(err)
        })?;

    tracing::info!("database migrations up to date");
    Ok(())
}
