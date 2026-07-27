use std::path::PathBuf;

use serde::Serialize;

/// Failure modes for watch-management operations
/// ([`crate::watcher::FileWatcher::watch`]/`unwatch`).
///
/// Kept separate from [`crate::errors::DatabaseError`] since these are
/// filesystem/watch-state failures (a bad path, an already-watched
/// directory), not storage failures — `commands::watcher`'s handlers need
/// to tell the two apart, even though a watch operation can also fail
/// for a database reason (persisting the updated watch list), which is
/// why this type wraps [`DatabaseError`] rather than duplicating it.
#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("'{0}' does not exist or is not a directory")]
    InvalidPath(PathBuf),

    #[error("'{0}' is already being watched")]
    AlreadyWatching(PathBuf),

    #[error("'{0}' is not currently being watched")]
    NotWatching(PathBuf),

    #[error(transparent)]
    Database(#[from] crate::errors::DatabaseError),
}

/// Manual [`Serialize`] impl, same rationale as
/// [`crate::errors::DatabaseError`]'s: lets this cross the Tauri IPC
/// boundary directly as the `E` in a command's `Result<T, E>`.
impl Serialize for WatcherError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
