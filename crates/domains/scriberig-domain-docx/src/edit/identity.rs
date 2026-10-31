//! Derives deterministic document identities from semantic parents and paths.

use scriberig_core::EntityId;

/// Derives stable document identities using the importer's namespace.
///
/// # Parameters
/// - `parent`: Stable semantic parent identity.
/// - `path`: Stable structural path.
///
/// # Returns
/// Deterministic Scriberig entity identity.
pub(super) fn derive_id(parent: EntityId, path: &str) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-docx-semantic-v1\0");
    hasher.update(parent.as_bytes());
    hasher.update(path.as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}
