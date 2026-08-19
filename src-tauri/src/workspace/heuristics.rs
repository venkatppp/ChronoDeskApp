//! Workspace boundary heuristics (blueprint §2.2 / Phase 3's "Workspace
//! Boundary Detection" objective).
//!
//! A workspace is not simply a folder — these heuristics look for marker
//! files/directories that indicate a directory is the *root* of a
//! meaningful unit of work (a git repo, a Node/Rust/Java/Python project)
//! rather than an arbitrary intermediate folder. Detection here is
//! synchronous, filesystem-only pattern matching — no ML. The blueprint's
//! Phase 5 ML Layer (clustering, similarity) is designed to sit on top of
//! this signal, not replace it.

use std::path::Path;

/// One filesystem signal indicating a directory is a workspace root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkspaceMarker {
    GitRepository,
    RustCrate,
    NodePackage,
    JavaMaven,
    JavaGradle,
    PythonProject,
    Readme,
}

impl WorkspaceMarker {
    /// Relative weight this marker contributes to a directory's
    /// confidence score. A git repository or a language-specific project
    /// manifest is a strong, near-certain signal; a bare `README` is weak
    /// on its own (plenty of subdirectories have one) but reinforces
    /// other signals when combined.
    pub fn weight(self) -> f64 {
        match self {
            WorkspaceMarker::GitRepository => 1.0,
            WorkspaceMarker::RustCrate => 0.9,
            WorkspaceMarker::NodePackage => 0.9,
            WorkspaceMarker::JavaMaven => 0.9,
            WorkspaceMarker::JavaGradle => 0.9,
            WorkspaceMarker::PythonProject => 0.7,
            WorkspaceMarker::Readme => 0.3,
        }
    }

    /// Human-readable label, used in logging and (eventually) surfaced to
    /// the user as "detected as: git repository, Rust crate".
    pub fn label(self) -> &'static str {
        match self {
            WorkspaceMarker::GitRepository => "git repository",
            WorkspaceMarker::RustCrate => "Rust crate",
            WorkspaceMarker::NodePackage => "Node package",
            WorkspaceMarker::JavaMaven => "Java (Maven) project",
            WorkspaceMarker::JavaGradle => "Java (Gradle) project",
            WorkspaceMarker::PythonProject => "Python project",
            WorkspaceMarker::Readme => "README present",
        }
    }
}

/// Minimum combined [`WorkspaceMarker::weight`] for a directory to be
/// treated as a workspace root. Any single strong marker (git, a
/// language manifest) clears this alone; a lone `README` does not.
pub const DETECTION_THRESHOLD: f64 = 0.8;

/// Inspects the immediate contents of `dir` — non-recursive; this checks
/// *this* directory's own marker files, not its subdirectories — and
/// returns every marker found. Never touches the filesystem outside
/// `dir` itself.
pub fn detect_markers(dir: &Path) -> Vec<WorkspaceMarker> {
    let mut markers = Vec::new();
    let has = |name: &str| dir.join(name).exists();

    if dir.join(".git").is_dir() {
        markers.push(WorkspaceMarker::GitRepository);
    }
    if has("Cargo.toml") {
        markers.push(WorkspaceMarker::RustCrate);
    }
    if has("package.json") {
        markers.push(WorkspaceMarker::NodePackage);
    }
    if has("pom.xml") {
        markers.push(WorkspaceMarker::JavaMaven);
    }
    if has("build.gradle") || has("build.gradle.kts") {
        markers.push(WorkspaceMarker::JavaGradle);
    }
    if has("pyproject.toml") || has("setup.py") {
        markers.push(WorkspaceMarker::PythonProject);
    }
    if ["README.md", "README", "README.txt", "Readme.md"]
        .iter()
        .any(|name| has(name))
    {
        markers.push(WorkspaceMarker::Readme);
    }

    markers
}

/// Combines marker weights into a single confidence score. Deliberately
/// unbounded above 1.0 — a directory that's both a git repository *and*
/// a Cargo crate is an even stronger signal than either alone, not
/// something to cap artificially.
pub fn confidence_score(markers: &[WorkspaceMarker]) -> f64 {
    markers.iter().map(|m| m.weight()).sum()
}

/// True if `markers` clears [`DETECTION_THRESHOLD`].
pub fn is_workspace_root(markers: &[WorkspaceMarker]) -> bool {
    confidence_score(markers) >= DETECTION_THRESHOLD
}

/// Suggests a human-readable workspace name from a root directory: its
/// own directory name (e.g. `/Users/me/projects/contextsphere` →
/// `"contextsphere"`), falling back to the full path for the rare case of a
/// directory with no file-name component (e.g. a filesystem root).
pub fn suggest_name(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_git_repository() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();

        let markers = detect_markers(dir.path());
        assert!(markers.contains(&WorkspaceMarker::GitRepository));
        assert!(is_workspace_root(&markers));
    }

    #[test]
    fn detects_rust_crate() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"").unwrap();

        let markers = detect_markers(dir.path());
        assert!(markers.contains(&WorkspaceMarker::RustCrate));
        assert!(is_workspace_root(&markers));
    }

    #[test]
    fn detects_node_package() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();

        let markers = detect_markers(dir.path());
        assert!(markers.contains(&WorkspaceMarker::NodePackage));
        assert!(is_workspace_root(&markers));
    }

    #[test]
    fn detects_java_maven_and_gradle_independently() {
        let maven_dir = tempdir().unwrap();
        fs::write(maven_dir.path().join("pom.xml"), "<project/>").unwrap();
        assert!(detect_markers(maven_dir.path()).contains(&WorkspaceMarker::JavaMaven));

        let gradle_dir = tempdir().unwrap();
        fs::write(gradle_dir.path().join("build.gradle.kts"), "").unwrap();
        assert!(detect_markers(gradle_dir.path()).contains(&WorkspaceMarker::JavaGradle));
    }

    #[test]
    fn lone_readme_is_not_enough_on_its_own() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("README.md"), "# hi").unwrap();

        let markers = detect_markers(dir.path());
        assert!(markers.contains(&WorkspaceMarker::Readme));
        assert!(
            !is_workspace_root(&markers),
            "a bare README should not be enough to declare a workspace root"
        );
    }

    #[test]
    fn readme_reinforces_a_strong_marker() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"").unwrap();
        fs::write(dir.path().join("README.md"), "# hi").unwrap();

        let markers = detect_markers(dir.path());
        assert!(confidence_score(&markers) > WorkspaceMarker::RustCrate.weight());
    }

    #[test]
    fn empty_directory_has_no_markers() {
        let dir = tempdir().unwrap();
        let markers = detect_markers(dir.path());
        assert!(markers.is_empty());
        assert!(!is_workspace_root(&markers));
    }

    #[test]
    fn suggest_name_uses_directory_basename() {
        let dir = tempdir().unwrap();
        let project_dir = dir.path().join("my-cool-project");
        fs::create_dir(&project_dir).unwrap();
        assert_eq!(suggest_name(&project_dir), "my-cool-project");
    }
}
