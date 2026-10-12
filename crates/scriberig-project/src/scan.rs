//! Implements deterministic project filesystem scanning and file classification.

use std::collections::BTreeSet;
use std::path::Path;

use crate::ScanFile;

/// Scans supported and unknown files below one project root.
///
/// # Parameters
/// - `root`: Project root to scan.
/// - `tracked`: Optional set of Git-indexed paths to retain.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Deterministically ordered scan files.
///
/// # Errors
/// Returns a filesystem traversal or read error.
pub(crate) fn scan_directory(
    root: &Path,
    tracked: Option<&BTreeSet<String>>,
) -> std::io::Result<Vec<ScanFile>> {
    let mut files = Vec::new();
    visit_directory(root, root, tracked, &mut files)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

/// Visits a directory recursively without following symlinks or generated trees.
///
/// # Parameters
/// - `root`: Project root used for relative paths.
/// - `directory`: Directory currently being visited.
/// - `tracked`: Optional set of paths selected by Git.
/// - `files`: Scan results being collected.
///
/// # Returns
/// Unit after the directory has been visited.
///
/// # Errors
/// Returns a filesystem traversal or read error.
fn visit_directory(
    root: &Path,
    directory: &Path,
    tracked: Option<&BTreeSet<String>>,
    files: &mut Vec<ScanFile>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if is_excluded(relative) || entry.file_type()?.is_symlink() {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            visit_directory(root, &path, tracked, files)?;
        } else if file_type.is_file() {
            let relative_path = relative.to_string_lossy().replace('\\', "/");
            if tracked.is_some_and(|paths| !paths.contains(&relative_path)) {
                continue;
            }
            let adapter = adapter_for(&path).to_owned();
            files.push(ScanFile {
                relative_path,
                bytes: std::fs::read(&path)?,
                status: if adapter == "unsupported" {
                    "unsupported".to_owned()
                } else {
                    "ready".to_owned()
                },
                adapter,
            });
        }
    }
    Ok(())
}

/// Returns whether a project-relative path is excluded by default.
///
/// # Parameters
/// - `path`: Relative path to classify.
///
/// # Returns
/// Whether the path belongs to a generated or hidden tree.
fn is_excluded(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some(
                ".git"
                    | "target"
                    | "node_modules"
                    | "dist"
                    | "build"
                    | ".pages"
                    | "__pycache__"
                    | ".venv"
                    | ".cache"
            )
        )
    })
}

/// Selects a supported source adapter by file extension.
///
/// # Parameters
/// - `path`: File path to classify.
///
/// # Returns
/// The selected adapter name.
fn adapter_for(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("html" | "htm") => "html",
        Some("c" | "h") => "c",
        Some("cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx") => "cpp",
        Some("js" | "mjs" | "cjs" | "jsx") => "javascript",
        Some("ts" | "mts" | "cts" | "tsx") => "typescript",
        Some("md" | "markdown") => "markdown",
        Some("py" | "pyi" | "pyw") => "python",
        Some("rs") => "rust",
        Some("pdf") => "pdf",
        _ => "unsupported",
    }
}
