//! Implements deterministic project filesystem scanning and file classification.

use std::collections::BTreeSet;
use std::path::Path;

use crate::ScanFile;

/// Scans supported and unknown files below one project root.
///
/// - `root`: Project root to scan.
/// - `tracked`: Optional set of Git-indexed paths to retain.
/// - `crate`: Internal visibility marker.
///
/// Deterministically ordered scan files.
///
/// Returns a filesystem traversal or read error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn scan_directory(
    root: &Path,
    tracked: Option<&BTreeSet<String>>,
) -> std::io::Result<Vec<ScanFile>> {
    let mut files = Vec::new();
    let sensitive = scriberig_secrets::sensitive::SensitivePaths::from_process()
        .map_err(std::io::Error::other)?;
    visit_directory(root, root, tracked, &mut files, &sensitive)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

/// Visits a directory recursively without following symlinks or generated trees.
///
/// - `root`: Project root used for relative paths.
/// - `directory`: Directory currently being visited.
/// - `tracked`: Optional set of paths selected by Git.
/// - `files`: Scan results being collected.
/// - `sensitive`: Credential paths excluded before reading any source bytes.
///
/// Unit after the directory has been visited.
///
/// Returns a filesystem traversal or read error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn visit_directory(
    root: &Path,
    directory: &Path,
    tracked: Option<&BTreeSet<String>>,
    files: &mut Vec<ScanFile>,
    sensitive: &scriberig_secrets::sensitive::SensitivePaths,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if is_excluded(relative) || sensitive.contains(&path) || entry.file_type()?.is_symlink() {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            visit_directory(root, &path, tracked, files, sensitive)?;
        } else if file_type.is_file() {
            let relative_path = relative.to_string_lossy().replace('\\', "/");
            if tracked.is_some_and(|paths| !paths.contains(&relative_path)) {
                continue;
            }
            let bytes = std::fs::read(&path)?;
            let adapter = adapter_for(&path, &bytes).to_owned();
            files.push(ScanFile {
                relative_path,
                bytes,
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
/// - `path`: Relative path to classify.
///
/// Whether the path belongs to a generated or hidden tree.
/// # Returns
/// The operation result.
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
/// - `path`: File path to classify.
/// - `bytes`: File bytes used to distinguish unknown text from binary data.
/// - `crate`: Internal visibility marker.
///
/// The selected adapter name.
/// # Returns
/// The operation result.
pub(crate) fn adapter_for(path: &Path, bytes: &[u8]) -> &'static str {
    let name = path.file_name().and_then(|value| value.to_str());
    if matches!(
        name,
        Some(
            "Dockerfile"
                | "Makefile"
                | "justfile"
                | "Taskfile"
                | "LICENSE"
                | "README"
                | ".gitignore"
                | ".gitattributes"
                | ".editorconfig"
                | ".gitmodules"
                | ".npmrc"
                | ".yarnrc"
                | ".prettierrc"
                | ".eslintrc"
                | ".babelrc"
                | ".flake8"
                | "gradlew"
                | "Gemfile"
                | "Rakefile"
                | "Brewfile"
                | "Vagrantfile"
                | "Procfile"
        )
    ) {
        return "text";
    }
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html" | "htm") => "html",
        Some("php") => "php",
        Some("c" | "h") => "c",
        Some("cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx") => "cpp",
        Some("js" | "mjs" | "cjs" | "jsx") => "javascript",
        Some("ts" | "mts" | "cts" | "tsx") => "typescript",
        Some("md" | "markdown") => "markdown",
        Some("py" | "pyi" | "pyw") => "python",
        Some("rs") => "rust",
        Some("pdf") => "pdf",
        Some(
            "sh" | "bash" | "zsh" | "fish" | "ps1" | "psm1" | "bat" | "cmd" | "mmd" | "mermaid"
            | "svg" | "txt" | "log" | "env" | "xml" | "ini" | "cfg" | "properties" | "csv" | "tsv"
            | "rst" | "adoc" | "tex" | "graphql" | "gql" | "proto" | "sql" | "go" | "java" | "kt"
            | "kts" | "cs" | "swift" | "rb" | "lua" | "dart" | "scala" | "m" | "mm" | "hs"
            | "ex" | "exs" | "erl" | "zig" | "lock" | "sum" | "mod" | "gradle",
        ) => "text",
        Some("json" | "jsonc" | "ipynb") => "json",
        Some("yaml" | "yml") => "yaml",
        Some("toml") => "toml",
        Some(
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "bmp" | "woff" | "woff2" | "ttf"
            | "otf" | "wasm" | "zip" | "gz" | "tar" | "7z" | "jar" | "exe" | "dll" | "o" | "a"
            | "so",
        ) => "binary",
        _ if std::str::from_utf8(bytes).is_ok() => "text",
        _ => "binary",
    }
}
