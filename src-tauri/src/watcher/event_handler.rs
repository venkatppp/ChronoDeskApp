//! Raw `notify::Event` normalization and ignore-path filtering.
//!
//! Two responsibilities, kept together because they both operate on the
//! same raw event before anything downstream (the debouncer, workspace
//! detection) ever sees it:
//! 1. [`is_ignored`] — drop paths inside VCS/dependency/build
//!    directories, OS metadata files, and editor temp files, so the
//!    watcher never generates timeline noise for `.git/`, `node_modules/`,
//!    `target/`, `.DS_Store`, Vim swap files, and the like.
//! 2. [`normalize`] — collapse `notify`'s (larger, platform-leaky)
//!    `EventKind` taxonomy down to [`DebouncedEventKind`]'s three
//!    variants, including treating a same-event rename
//!    (`ModifyKind::Name(RenameMode::Both)`) as a remove-then-create pair
//!    at the two paths involved.

use std::path::{Path, PathBuf};

use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind};

use super::debounce::DebouncedEventKind;

/// Directory names never watched/recorded, regardless of where they
/// appear in a watched tree — build output and VCS/dependency
/// directories generate enormous, uninteresting event volume and are
/// explicitly called out by the blueprint's ignore-list requirement.
///
/// This is the single source of truth for generated/build/dependency
/// exclusions; the timeline recorder applies the same filter
/// (`crate::watcher::event_handler::is_ignored`) as defense-in-depth so
/// no layer (timeline, search, knowledge graph, semantic, analytics)
/// can ingest these paths.
const IGNORED_DIR_NAMES: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    "out",
    ".cache",
    "coverage",
    ".next",
    ".turbo",
    ".parcel-cache",
    ".vite",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
    ".pytest_cache",
    ".mypy_cache",
];

/// True if `path` should never generate a timeline event: it sits inside
/// an ignored directory, is a known OS metadata file, is a dotfile, or
/// looks like a transient editor/OS temp file.
pub fn is_ignored(path: &Path) -> bool {
    let in_ignored_dir = path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|name| IGNORED_DIR_NAMES.contains(&name))
    });
    if in_ignored_dir {
        return true;
    }

    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };

    if matches!(file_name, ".DS_Store" | "Thumbs.db" | "desktop.ini") {
        return true;
    }

    // Any dotfile/hidden file — covers editor config, `.env`, lockfiles,
    // etc. that are almost never meaningful "project work" on their own.
    if file_name.starts_with('.') {
        return true;
    }

    // Common editor/OS temp-file patterns: backup files, Vim swap files,
    // and files still mid-write (a trailing `~`/`.tmp`, or Office's
    // leading `~$` lock-file prefix).
    if file_name.ends_with('~')
        || file_name.ends_with(".tmp")
        || file_name.ends_with(".swp")
        || file_name.ends_with(".swx")
        || file_name.starts_with("~$")
    {
        return true;
    }

    false
}

/// Normalizes a raw `notify::Event` into zero or more `(path, kind)`
/// pairs ready for [`super::debounce::Debouncer::push`], after dropping
/// every ignored path.
///
/// `notify::Event` can carry multiple paths — a same-event rename
/// (`ModifyKind::Name(RenameMode::Both)`) carries both the old and new
/// path — so this returns a `Vec` rather than a single optional pair.
/// Event kinds this watcher doesn't act on (metadata-only changes,
/// access events, and rename events that only report the old *or* new
/// path in isolation rather than as a `Both` pair — a platform-dependent
/// case `notify` itself documents as best-effort) normalize to an empty
/// result rather than a guess.
pub fn normalize(event: &Event) -> Vec<(PathBuf, DebouncedEventKind)> {
    let raw: Vec<(PathBuf, DebouncedEventKind)> = match &event.kind {
        EventKind::Create(_) => event
            .paths
            .iter()
            .cloned()
            .map(|p| (p, DebouncedEventKind::Created))
            .collect(),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) if event.paths.len() == 2 => {
            vec![
                (event.paths[0].clone(), DebouncedEventKind::Removed),
                (event.paths[1].clone(), DebouncedEventKind::Created),
            ]
        }
        EventKind::Modify(_) => event
            .paths
            .iter()
            .cloned()
            .map(|p| (p, DebouncedEventKind::Modified))
            .collect(),
        EventKind::Remove(_) => event
            .paths
            .iter()
            .cloned()
            .map(|p| (p, DebouncedEventKind::Removed))
            .collect(),
        _ => Vec::new(),
    };

    raw.into_iter()
        .filter(|(path, _)| !is_ignored(path))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::CreateKind;

    #[test]
    fn ignores_paths_inside_dot_git() {
        assert!(is_ignored(Path::new("/repo/.git/HEAD")));
    }

    #[test]
    fn ignores_paths_inside_node_modules_and_target() {
        assert!(is_ignored(Path::new("/repo/node_modules/lib/index.js")));
        assert!(is_ignored(Path::new("/repo/target/debug/app")));
    }

    #[test]
    fn ignores_os_metadata_files() {
        assert!(is_ignored(Path::new("/repo/.DS_Store")));
        assert!(is_ignored(Path::new("/repo/Thumbs.db")));
    }

    #[test]
    fn ignores_editor_temp_files() {
        assert!(is_ignored(Path::new("/repo/src/main.rs~")));
        assert!(is_ignored(Path::new("/repo/src/.main.rs.swp")));
        assert!(is_ignored(Path::new("/repo/~$document.docx")));
    }

    #[test]
    fn does_not_ignore_ordinary_project_files() {
        assert!(!is_ignored(Path::new("/repo/src/main.rs")));
        assert!(!is_ignored(Path::new("/repo/README.md")));
        assert!(!is_ignored(Path::new("/repo/package.json")));
    }

    #[test]
    fn normalize_maps_create_event_to_created() {
        let event =
            Event::new(EventKind::Create(CreateKind::File)).add_path(PathBuf::from("/repo/new.rs"));

        let result = normalize(&event);
        assert_eq!(
            result,
            vec![(PathBuf::from("/repo/new.rs"), DebouncedEventKind::Created)]
        );
    }

    #[test]
    fn normalize_drops_ignored_paths() {
        let event = Event::new(EventKind::Create(CreateKind::File))
            .add_path(PathBuf::from("/repo/node_modules/x.js"));

        assert!(normalize(&event).is_empty());
    }

    #[test]
    fn normalize_maps_a_same_event_rename_to_remove_then_create() {
        let event = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(PathBuf::from("/repo/old.rs"))
            .add_path(PathBuf::from("/repo/new.rs"));

        let result = normalize(&event);
        assert_eq!(
            result,
            vec![
                (PathBuf::from("/repo/old.rs"), DebouncedEventKind::Removed),
                (PathBuf::from("/repo/new.rs"), DebouncedEventKind::Created),
            ]
        );
    }
}
