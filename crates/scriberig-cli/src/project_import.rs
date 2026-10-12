//! Implements filesystem scanning and immutable project import.

use std::path::Path;

use scriberig_core::{EncodingVersion, EntityId, ObjectType, ProjectId, RootName, SnapshotId};
use scriberig_store::{Revision, Snapshot, SnapshotChange};
use scriberig_store_sqlite::{ProjectRecord, ProjectStatus, SqliteStore};

/// Describes one filesystem file discovered during onboarding.
#[derive(Debug)]
pub(crate) struct ScannedFile {
    /// Project-relative POSIX path.
    pub(crate) relative_path: String,
    /// Exact source bytes.
    pub(crate) bytes: Vec<u8>,
    /// Selected domain adapter.
    pub(crate) adapter: &'static str,
}

impl ScannedFile {
    /// Converts a scanned file to the public review manifest representation.
    ///
    /// # Parameters
    /// - `self`: Scanned file to render.
    /// - `crate`: Internal visibility marker.
    ///
    /// # Returns
    /// A stable JSON manifest entry.
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.relative_path,
            "bytes": self.bytes.len(),
            "digest": blake3::hash(&self.bytes).to_hex().to_string(),
            "adapter": self.adapter,
            "status": if self.adapter == "unsupported" { "unsupported" } else { "ready" },
        })
    }
}

/// Scans supported and unknown files and retains bytes for import.
///
/// # Parameters
/// - `path`: Project directory to scan.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Deterministically ordered discovered files.
///
/// # Errors
/// Returns a filesystem traversal or read error.
pub(crate) fn scan_files(path: &Path) -> std::io::Result<Vec<ScannedFile>> {
    let mut files = Vec::new();
    scan_directory(path, path, &mut files)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

/// Recursively visits one directory while applying default exclusions.
///
/// # Parameters
/// - `root`: Project root used for relative paths.
/// - `directory`: Directory currently being visited.
/// - `files`: Scan results being collected.
///
/// # Returns
/// Unit after the directory has been visited.
///
/// # Errors
/// Returns a filesystem traversal or read error.
fn scan_directory(
    root: &Path,
    directory: &Path,
    files: &mut Vec<ScannedFile>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if is_excluded(relative) {
            continue;
        }
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            scan_directory(root, &path, files)?;
        } else if metadata.is_file() {
            files.push(ScannedFile {
                relative_path: relative.to_string_lossy().replace('\\', "/"),
                bytes: std::fs::read(&path)?,
                adapter: adapter_for(&path),
            });
        }
    }
    Ok(())
}

/// Returns whether a relative path belongs to a default-excluded directory.
///
/// # Parameters
/// - `path`: Relative path to classify.
///
/// # Returns
/// Whether the path is excluded.
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

/// Selects the first supported adapter by file extension.
///
/// # Parameters
/// - `path`: File path to classify.
///
/// # Returns
/// The adapter name.
fn adapter_for(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("md" | "markdown") => "markdown",
        Some("rs") => "rust",
        Some("pdf") => "pdf",
        _ => "unsupported",
    }
}

/// Imports supported files as durable project-scoped source artifacts.
///
/// # Parameters
/// - `store`: Durable project store.
/// - `project`: Project catalog record receiving the import.
/// - `files`: Files selected by the latest scan.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Number of imported files and the promoted snapshot.
///
/// # Errors
/// Returns validation, revision, or snapshot-promotion errors.
pub(crate) fn import_files(
    store: &SqliteStore,
    project: &ProjectRecord,
    files: &[ScannedFile],
) -> Result<(usize, SnapshotId), Box<dyn std::error::Error + Send + Sync>> {
    let base = project
        .current_snapshot
        .unwrap_or_else(|| Snapshot::empty().id());
    let root = RootName::new(format!("project_{}", project.id))?;
    let mut revisions = Vec::new();
    let mut changes = Vec::new();
    for file in files.iter().filter(|file| file.adapter != "unsupported") {
        validate_file(file)?;
        let entity = file_entity(project.id, &file.relative_path);
        let object_type = ObjectType::new(format!("project.file.{}", file.adapter))?;
        let revision = Revision::new(EncodingVersion::V1, object_type, file.bytes.clone())
            .map_err(|error| format!("revision creation failed: {error:?}"))?;
        changes.push(SnapshotChange::Set(entity, revision.id()));
        revisions.push(revision);
    }
    let snapshot = store
        .apply_revisions_and_promote(root, None, base, &revisions, &changes)
        .map_err(|error| format!("project import promotion failed: {error:?}"))?;
    store.set_project_status(project.id, ProjectStatus::Imported, Some(snapshot))?;
    Ok((changes.len(), snapshot))
}

/// Validates source text before it enters an immutable project snapshot.
///
/// # Parameters
/// - `file`: Source file to validate.
///
/// # Returns
/// Unit when the source is valid.
///
/// # Errors
/// Returns a parse or encoding error for invalid source.
fn validate_file(file: &ScannedFile) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match file.adapter {
        "markdown" => {
            let source = std::str::from_utf8(&file.bytes)?;
            if let Err(error) = sr_markdown::parse(source) {
                if error.code != "unsupported-markdown" {
                    return Err(format!("{}: {error:?}", file.relative_path).into());
                }
            }
        }
        "rust" => {
            std::str::from_utf8(&file.bytes)
                .map_err(|error| format!("{}: {error}", file.relative_path))?;
        }
        "pdf" if file.bytes.len() < 5 || &file.bytes[..5] != b"%PDF-" => {
            return Err(format!("{}: invalid PDF header", file.relative_path).into());
        }
        _ => {}
    }
    Ok(())
}

/// Derives a stable entity identity from a project and relative path.
///
/// # Parameters
/// - `project`: Stable project identity.
/// - `path`: Project-relative file path.
///
/// # Returns
/// Stable entity identity for the source artifact.
fn file_entity(project: ProjectId, path: &str) -> EntityId {
    let mut input = Vec::with_capacity(16 + path.len());
    input.extend_from_slice(project.as_bytes());
    input.extend_from_slice(path.as_bytes());
    let digest = blake3::hash(&input);
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}
