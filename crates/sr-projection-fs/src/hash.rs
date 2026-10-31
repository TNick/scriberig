//! Defines exact-byte filesystem content digests.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// BLAKE3 digest of exact file bytes.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct FileDigest(pub [u8; 32]);

/// Hashes exact in-memory bytes.
///
/// - `bytes`: Exact bytes to hash.
///
/// The BLAKE3 file digest.
#[must_use]
/// # Returns
/// The operation result.
pub fn digest_bytes(bytes: &[u8]) -> FileDigest {
    FileDigest(*blake3::hash(bytes).as_bytes())
}

/// Reads and hashes exact file bytes.
///
/// - `path`: File to read.
///
/// The exact-byte digest.
///
/// Returns the filesystem read error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn digest_file(path: impl AsRef<Path>) -> std::io::Result<FileDigest> {
    Ok(digest_bytes(&std::fs::read(path)?))
}
