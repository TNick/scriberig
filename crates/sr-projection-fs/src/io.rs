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
    OwnershipDenied,
}
impl std::fmt::Display for IoError {
    /// Formats an IO error.
    ///
    /// # Parameters
    /// - `self`: Error to format.
    /// - `f`: Formatter destination.
    ///
    /// # Returns
    /// Formatting result.
    ///
    /// # Errors
    /// Returns a formatting error when writing fails.
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
    /// # Parameters
    /// - `self`: Filesystem implementation.
    /// - `path`: File path.
    ///
    /// # Returns
    /// Exact file bytes.
    ///
    /// # Errors
    /// Returns an IO error when reading fails.
    fn read(&self, path: &Path) -> Result<Vec<u8>, IoError>;
    /// Returns metadata when a path exists.
    /// Reads existence metadata.
    ///
    /// # Parameters
    /// - `self`: Filesystem implementation.
    /// - `path`: File path.
    ///
    /// # Returns
    /// Metadata when the path exists.
    ///
    /// # Errors
    /// Returns an IO error for metadata failures.
    /// Reads local metadata.
    ///
    /// # Parameters
    /// - `self`: Local filesystem.
    /// - `path`: File path.
    ///
    /// # Returns
    /// Optional file metadata.
    ///
    /// # Errors
    /// Returns an IO error when metadata lookup fails.
    fn metadata(&self, path: &Path) -> Result<Option<FileMetadata>, IoError>;
    /// Replaces a target with exact bytes using a sibling temporary file.
    /// Replaces a file with exact bytes.
    ///
    /// # Parameters
    /// - `self`: Filesystem implementation.
    /// - `path`: Target path.
    /// - `bytes`: Exact replacement bytes.
    ///
    /// # Returns
    /// Unit after replacement.
    ///
    /// # Errors
    /// Returns an IO error when writing or replacing fails.
    /// Atomically replaces local bytes.
    ///
    /// # Parameters
    /// - `self`: Local filesystem.
    /// - `path`: Target path.
    /// - `bytes`: Replacement bytes.
    ///
    /// # Returns
    /// Unit after replacement.
    ///
    /// # Errors
    /// Returns an IO error when replacement fails.
    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), IoError>;
}

impl<F: FileSystem + ?Sized> FileSystem for &F {
    /// Reads exact bytes through a borrowed filesystem implementation.
    ///
    /// # Parameters
    /// - `self`: Borrowed filesystem implementation.
    /// - `path`: File path.
    ///
    /// # Returns
    /// Exact file bytes.
    ///
    /// # Errors
    /// Returns the delegated filesystem error.
    fn read(&self, path: &Path) -> Result<Vec<u8>, IoError> {
        (**self).read(path)
    }

    /// Reads metadata through a borrowed filesystem implementation.
    ///
    /// # Parameters
    /// - `self`: Borrowed filesystem implementation.
    /// - `path`: File path.
    ///
    /// # Returns
    /// Optional file metadata.
    ///
    /// # Errors
    /// Returns the delegated filesystem error.
    fn metadata(&self, path: &Path) -> Result<Option<FileMetadata>, IoError> {
        (**self).metadata(path)
    }

    /// Atomically writes bytes through a borrowed filesystem implementation.
    ///
    /// # Parameters
    /// - `self`: Borrowed filesystem implementation.
    /// - `path`: Target path.
    /// - `bytes`: Exact replacement bytes.
    ///
    /// # Returns
    /// Unit after replacement.
    ///
    /// # Errors
    /// Returns the delegated filesystem error.
    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), IoError> {
        (**self).write_atomic(path, bytes)
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
    /// # Parameters
    /// - `self`: Local filesystem.
    /// - `path`: File path.
    ///
    /// # Returns
    /// Exact bytes.
    ///
    /// # Errors
    /// Returns an IO error when reading fails.
    fn read(&self, path: &Path) -> Result<Vec<u8>, IoError> {
        std::fs::read(path).map_err(IoError::Read)
    }
    /// Reads local metadata.
    ///
    /// # Parameters
    /// - `self`: Local filesystem.
    /// - `path`: File path.
    ///
    /// # Returns
    /// Optional file metadata.
    ///
    /// # Errors
    /// Returns an IO error when metadata lookup fails.
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
    /// # Parameters
    /// - `self`: Local filesystem.
    /// - `path`: Target path.
    /// - `bytes`: Replacement bytes.
    ///
    /// # Returns
    /// Unit after replacement.
    ///
    /// # Errors
    /// Returns an IO error when replacement fails.
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
}
