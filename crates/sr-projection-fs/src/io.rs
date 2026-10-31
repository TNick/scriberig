//! Defines replace-safe local filesystem IO.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Structured local IO failure.
#[derive(Debug)]
pub enum IoError {
    Read(std::io::Error),
    Write(std::io::Error),
    Rename(std::io::Error),
    Directory(std::io::Error),
    Remove(std::io::Error),
    OwnershipDenied,
    BaselineMismatch,
    /// Artifact materialization plan has empty, unsafe, or duplicate members.
    UnsafeArtifactPlan,
}
impl std::fmt::Display for IoError {
    /// Formats an IO error.
    ///
    /// - `self`: Error to format.
    /// - `f`: Formatter destination.
    ///
    /// Formatting result.
    ///
    /// Returns a formatting error when writing fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for IoError {}

/// Basic filesystem operations required by projection synchronization.
pub trait FileSystem {
    /// Reads exact bytes.
    /// Reads exact bytes.
    ///
    /// - `self`: Filesystem implementation.
    /// - `path`: File path.
    ///
    /// Exact file bytes.
    ///
    /// Returns an IO error when reading fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read(&self, path: &Path) -> Result<Vec<u8>, IoError>;
    /// Returns metadata when a path exists.
    /// Reads existence metadata.
    ///
    /// - `self`: Filesystem implementation.
    /// - `path`: File path.
    ///
    /// Metadata when the path exists.
    ///
    /// Returns an IO error for metadata failures.
    /// Reads local metadata.
    ///
    /// - `self`: Local filesystem.
    /// - `path`: File path.
    ///
    /// Optional file metadata.
    ///
    /// Returns an IO error when metadata lookup fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn metadata(&self, path: &Path) -> Result<Option<FileMetadata>, IoError>;
    /// Replaces a target with exact bytes using a sibling temporary file.
    /// Replaces a file with exact bytes.
    ///
    /// - `self`: Filesystem implementation.
    /// - `path`: Target path.
    /// - `bytes`: Exact replacement bytes.
    ///
    /// Unit after replacement.
    ///
    /// Returns an IO error when writing or replacing fails.
    /// Atomically replaces local bytes.
    ///
    /// - `self`: Local filesystem.
    /// - `path`: Target path.
    /// - `bytes`: Replacement bytes.
    ///
    /// Unit after replacement.
    ///
    /// Returns an IO error when replacement fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), IoError>;
    /// Creates missing parent directories for an authorized materialization.
    ///
    /// - `self`: Filesystem implementation.
    /// - `path`: Directory path to create.
    ///
    /// Unit after the directory exists.
    ///
    /// Returns an IO error when directory creation fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn create_dir_all(&self, path: &Path) -> Result<(), IoError>;
    /// Removes one authorized file after caller precondition checks.
    ///
    /// - `self`: Filesystem implementation.
    /// - `path`: File path to remove.
    ///
    /// Unit after the file is removed.
    ///
    /// Returns an IO error when removal fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn remove_file(&self, path: &Path) -> Result<(), IoError>;
}

impl<F: FileSystem + ?Sized> FileSystem for &F {
    /// Reads exact bytes through a borrowed filesystem implementation.
    ///
    /// - `self`: Borrowed filesystem implementation.
    /// - `path`: File path.
    ///
    /// Exact file bytes.
    ///
    /// Returns the delegated filesystem error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read(&self, path: &Path) -> Result<Vec<u8>, IoError> {
        (**self).read(path)
    }

    /// Reads metadata through a borrowed filesystem implementation.
    ///
    /// - `self`: Borrowed filesystem implementation.
    /// - `path`: File path.
    ///
    /// Optional file metadata.
    ///
    /// Returns the delegated filesystem error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn metadata(&self, path: &Path) -> Result<Option<FileMetadata>, IoError> {
        (**self).metadata(path)
    }

    /// Atomically writes bytes through a borrowed filesystem implementation.
    ///
    /// - `self`: Borrowed filesystem implementation.
    /// - `path`: Target path.
    /// - `bytes`: Exact replacement bytes.
    ///
    /// Unit after replacement.
    ///
    /// Returns the delegated filesystem error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), IoError> {
        (**self).write_atomic(path, bytes)
    }

    /// Creates missing directories through the borrowed filesystem.
    ///
    /// - `self`: Borrowed filesystem implementation.
    /// - `path`: Directory path to create.
    ///
    /// Unit after directory creation.
    ///
    /// Returns the delegated IO error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn create_dir_all(&self, path: &Path) -> Result<(), IoError> {
        (**self).create_dir_all(path)
    }

    /// Removes one file through the borrowed filesystem.
    ///
    /// - `self`: Borrowed filesystem implementation.
    /// - `path`: File path to remove.
    ///
    /// Unit after removal.
    ///
    /// Returns the delegated IO error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn remove_file(&self, path: &Path) -> Result<(), IoError> {
        (**self).remove_file(path)
    }
}

/// Minimal filesystem metadata exposed without watcher types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileMetadata {
    pub exists: bool,
    pub length: u64,
}

/// Local operating-system filesystem implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalFileSystem;

impl FileSystem for LocalFileSystem {
    /// Reads exact local bytes.
    ///
    /// - `self`: Local filesystem.
    /// - `path`: File path.
    ///
    /// Exact bytes.
    ///
    /// Returns an IO error when reading fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read(&self, path: &Path) -> Result<Vec<u8>, IoError> {
        std::fs::read(path).map_err(IoError::Read)
    }
    /// Reads local metadata.
    ///
    /// - `self`: Local filesystem.
    /// - `path`: File path.
    ///
    /// Optional file metadata.
    ///
    /// Returns an IO error when metadata lookup fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn metadata(&self, path: &Path) -> Result<Option<FileMetadata>, IoError> {
        match std::fs::metadata(path) {
            Ok(m) => Ok(Some(FileMetadata {
                exists: true,
                length: m.len(),
            })),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(IoError::Read(e)),
        }
    }
    /// Atomically replaces local bytes.
    ///
    /// - `self`: Local filesystem.
    /// - `path`: Target path.
    /// - `bytes`: Replacement bytes.
    ///
    /// Unit after replacement.
    ///
    /// Returns an IO error when replacement fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), IoError> {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("projection");
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            ".{name}.scriberig-tmp-{}-{sequence}",
            std::process::id()
        ));
        std::fs::write(&temp, bytes).map_err(IoError::Write)?;
        if let Err(e) = std::fs::rename(&temp, path) {
            let _ = std::fs::remove_file(&temp);
            return Err(IoError::Rename(e));
        }
        Ok(())
    }

    /// Creates a local directory tree.
    ///
    /// - `self`: Local filesystem.
    /// - `path`: Directory path to create.
    ///
    /// Unit after the directory exists.
    ///
    /// Returns an IO error when creation fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn create_dir_all(&self, path: &Path) -> Result<(), IoError> {
        std::fs::create_dir_all(path).map_err(IoError::Directory)
    }

    /// Removes one local file.
    ///
    /// - `self`: Local filesystem.
    /// - `path`: File path to remove.
    ///
    /// Unit after removal.
    ///
    /// Returns an IO error when removal fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn remove_file(&self, path: &Path) -> Result<(), IoError> {
        std::fs::remove_file(path).map_err(IoError::Remove)
    }
}
