//! Creates stable child identities for fixture entities.

use super::*;

/// Derives a stable child entity under one source artifact.
///
/// - `source`: Parent source entity.
/// - `kind`: Child entity kind.
/// - `index`: Child ordinal.
///
/// Stable child entity identity.
/// # Returns
/// The operation result.
pub(super) fn child_entity(source: EntityId, kind: &str, index: usize) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(source.as_bytes());
    hasher.update(kind.as_bytes());
    hasher.update(&index.to_be_bytes());
    let mut identity = [0; 16];
    identity.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(identity)
}
