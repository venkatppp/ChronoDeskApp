//! Workspace root detection: given a file path within a watched
//! directory, finds the nearest ancestor directory that looks like a
//! workspace root (blueprint §2.2).

use std::path::{Path, PathBuf};

use super::heuristics::{self, WorkspaceMarker};

/// A directory the detector has determined is a workspace root, along
/// with the markers that justified the decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedWorkspaceRoot {
    pub path: PathBuf,
    pub markers: Vec<WorkspaceMarker>,
    pub suggested_name: String,
}

/// Walks upward from `file_path`'s parent directory toward (and
/// including) `watch_root`, returning the *nearest* ancestor directory
/// that clears [`heuristics::is_workspace_root`], or `None` if nothing in
/// that range qualifies. The walk never climbs above `watch_root` — the
/// detector only ever considers directories the user opted into
/// watching, never anything above it.
///
/// "Nearest" rather than "outermost" is the deliberate choice: for the
/// overwhelmingly common case — one project, one repo, worked on from
/// its own root — the closest qualifying ancestor *is* the repo root,
/// since intermediate subdirectories like `src/` carry no markers of
/// their own. In a monorepo, the nearest nested `Cargo.toml`/
/// `package.json` is detected as its own workspace, which is arguably
/// correct too: each nested manifest is its own unit of work.
///
/// # Panics
/// Never panics. Directories that no longer exist (e.g. a delete event
/// removed the whole tree) simply fail their marker checks and are
/// skipped, same as a directory with no markers.
pub fn detect_workspace_root(file_path: &Path, watch_root: &Path) -> Option<DetectedWorkspaceRoot> {
    let mut current = file_path.parent();

    while let Some(dir) = current {
        if !dir.starts_with(watch_root) {
            break;
        }

        let markers = heuristics::detect_markers(dir);
        if heuristics::is_workspace_root(&markers) {
            return Some(DetectedWorkspaceRoot {
                path: dir.to_path_buf(),
                suggested_name: heuristics::suggest_name(dir),
                markers,
            });
        }

        if dir == watch_root {
            break;
        }
        current = dir.parent();
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn finds_repo_root_from_a_nested_file() {
        let watch_root = tempdir().unwrap();
        fs::create_dir(watch_root.path().join(".git")).unwrap();
        let src_dir = watch_root.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        let file_path = src_dir.join("main.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let detected = detect_workspace_root(&file_path, watch_root.path())
            .expect("should detect the watch root itself as the workspace root");

        assert_eq!(detected.path, watch_root.path());
        assert!(detected.markers.contains(&WorkspaceMarker::GitRepository));
    }

    #[test]
    fn finds_nearest_nested_project_in_a_monorepo() {
        let watch_root = tempdir().unwrap();
        // watch_root has no markers of its own.
        let nested_project = watch_root.path().join("packages/api");
        fs::create_dir_all(&nested_project).unwrap();
        fs::write(nested_project.join("package.json"), "{}").unwrap();
        let file_path = nested_project.join("src/index.js");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "").unwrap();

        let detected = detect_workspace_root(&file_path, watch_root.path())
            .expect("should detect the nested package as its own workspace");

        assert_eq!(detected.path, nested_project);
        assert!(detected.markers.contains(&WorkspaceMarker::NodePackage));
    }

    #[test]
    fn returns_none_when_nothing_in_range_qualifies() {
        let watch_root = tempdir().unwrap();
        let loose_file = watch_root.path().join("random-notes.txt");
        fs::write(&loose_file, "just some notes").unwrap();

        assert!(detect_workspace_root(&loose_file, watch_root.path()).is_none());
    }

    #[test]
    fn never_climbs_above_watch_root() {
        // watch_root itself has no markers; its *parent* (the tempdir
        // above it) might coincidentally contain unrelated files, but the
        // detector must never look there.
        let outer = tempdir().unwrap();
        let watch_root = outer.path().join("watched");
        fs::create_dir(&watch_root).unwrap();
        let file_path = watch_root.join("notes.md");
        fs::write(&file_path, "").unwrap();

        assert!(detect_workspace_root(&file_path, &watch_root).is_none());
    }
}
