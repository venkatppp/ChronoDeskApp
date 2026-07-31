//! Duplicate detection models (Phase 5 Stage 2).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::FileArtifact;

/// A group of duplicate files sharing the same content hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    /// The SHA-256 content hash shared by all files in this group.
    pub content_hash: String,
    /// Files in this duplicate group (2 or more).
    pub files: Vec<DuplicateFile>,
    /// Number of files in the group.
    pub file_count: usize,
    /// Total size in bytes of all files in the group.
    /// This represents wasted disk space (size * (count - 1)).
    pub total_size: u64,
}

/// A file within a duplicate group, with additional metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFile {
    /// The file's unique identifier.
    pub id: Uuid,
    /// The workspace this file belongs to.
    pub workspace_id: Uuid,
    /// File path or URL.
    pub path_or_url: String,
    /// File size in bytes (if available).
    pub size: Option<u64>,
    /// The content hash (redundant with group, but useful for display).
    pub content_hash: String,
}

impl DuplicateFile {
    /// Creates a DuplicateFile from a FileArtifact.
    ///
    /// Note: `size` is set to None since FileArtifact doesn't track size yet.
    /// Future enhancement: add size column to files table.
    pub fn from_artifact(artifact: FileArtifact) -> Self {
        Self {
            id: artifact.id,
            workspace_id: artifact.workspace_id,
            path_or_url: artifact.path_or_url,
            size: None, // TODO: Add size tracking to files table
            content_hash: artifact.content_hash.unwrap_or_default(),
        }
    }
}

/// Progress information for a duplicate detection scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    /// Number of files successfully hashed so far.
    pub files_scanned: usize,
    /// Total number of files to scan.
    pub total_files: usize,
    /// Path of the file currently being hashed (if any).
    pub current_file: Option<String>,
    /// Number of files that failed to hash (locked, deleted, permission denied).
    pub files_failed: usize,
    /// Whether the scan is complete.
    pub is_complete: bool,
}

impl ScanProgress {
    /// Creates a new progress tracker.
    pub fn new(total_files: usize) -> Self {
        Self {
            files_scanned: 0,
            total_files,
            current_file: None,
            files_failed: 0,
            is_complete: false,
        }
    }

    /// Updates progress after successfully hashing a file.
    pub fn increment_scanned(&mut self, file_path: String) {
        self.files_scanned += 1;
        self.current_file = Some(file_path);
    }

    /// Updates progress after a file fails to hash.
    pub fn increment_failed(&mut self) {
        self.files_failed += 1;
    }

    /// Marks the scan as complete.
    pub fn mark_complete(&mut self) {
        self.is_complete = true;
        self.current_file = None;
    }

    /// Returns the percentage complete (0-100).
    pub fn percentage(&self) -> f32 {
        if self.total_files == 0 {
            return 100.0;
        }
        (self.files_scanned as f32 / self.total_files as f32) * 100.0
    }
}
