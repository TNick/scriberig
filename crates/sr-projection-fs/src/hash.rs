//! Defines exact-byte filesystem content digests.

use std::path::Path;

/// BLAKE3 digest of exact file bytes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FileDigest(pub [u8; 32]);

/// Hashes exact in-memory bytes.
///
/// # Parameters
/// - `bytes`: Exact bytes to hash.
///
/// # Returns
/// The BLAKE3 file digest.
#[must_use]
pub fn digest_bytes(bytes: &[u8]) -> FileDigest {
    FileDigest(*blake3::hash(bytes).as_bytes())
}

/// Reads and hashes exact file bytes.
///
/// # Parameters
/// - `path`: File to read.
///
/// # Returns
/// The exact-byte digest.
///
/// # Errors
/// Returns the filesystem read error.
pub fn digest_file(path: impl AsRef<Path>) -> std::io::Result<FileDigest> {
    Ok(digest_bytes(&std::fs::read(path)?))
}
