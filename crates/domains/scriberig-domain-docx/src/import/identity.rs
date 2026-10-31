//! Derives stable identities for imported DOCX structures.

use scriberig_core::EntityId;

/// Namespace carrying persistent `WordprocessingML` paragraph identities.
pub(super) const WORD_2010_NAMESPACE: &str = "http://schemas.microsoft.com/office/word/2010/wordml";

/// Derives a deterministic semantic identity from a parent and source path.
///
/// # Parameters
/// - `parent`: Stable parent identity.
/// - `path`: Stable source-order path.
///
/// # Returns
/// Deterministic document identity.
pub(super) fn derive_id(parent: EntityId, path: &str) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-docx-semantic-v1\0");
    hasher.update(parent.as_bytes());
    hasher.update(path.as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}

/// Derives a paragraph identity from a persistent Word paragraph ID.
///
/// # Parameters
/// - `document`: Stable semantic document identity.
/// - `value`: Eight-digit hexadecimal `w14:paraId`.
///
/// # Returns
/// Paragraph identity independent of XML position.
pub(super) fn persistent_paragraph_id(document: EntityId, value: &str) -> EntityId {
    derive_id(
        document,
        &format!("paragraph/w14/{}", value.to_ascii_uppercase()),
    )
}

/// Validates the fixed-width hexadecimal representation of a Word paragraph ID.
///
/// # Parameters
/// - `value`: `w14:paraId` attribute value.
///
/// # Returns
/// True when the value is exactly eight hexadecimal digits.
pub(super) fn valid_para_id(value: &str) -> bool {
    value.len() == 8 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
