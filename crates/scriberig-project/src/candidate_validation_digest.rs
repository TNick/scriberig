//! Computes exact output digests used to bind validation to candidate effects.

use blake3::Hasher;

use crate::ProjectSourceEffect;

/// Hashes ordered candidate outputs and their project-relative paths.
///
/// Deletes contribute their path and a distinct absent-output marker. Length
/// prefixes make concatenated paths and byte outputs unambiguous.
///
/// # Parameters
/// - `effects`: Exact ordered source effects being validated.
///
/// # Returns
/// Stable digest for the complete candidate output set.
pub(crate) fn candidate_validation_digest(effects: &[ProjectSourceEffect]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(b"scriberig-candidate-validation-output-v1\0");
    for effect in effects {
        update_length_prefixed(&mut hasher, effect.path.as_bytes());
        hasher.update(&[match effect.kind {
            crate::ProjectSourceEffectKind::Create => 1,
            crate::ProjectSourceEffectKind::Update => 2,
            crate::ProjectSourceEffectKind::Delete => 3,
        }]);
        match &effect.new {
            Some(bytes) => {
                hasher.update(&[1]);
                update_length_prefixed(&mut hasher, bytes);
            }
            None => {
                hasher.update(&[0]);
            }
        }
    }
    *hasher.finalize().as_bytes()
}

/// Adds one length-prefixed byte sequence to a digest.
///
/// # Parameters
/// - `hasher`: Digest state receiving the sequence.
/// - `bytes`: Exact byte sequence to append.
fn update_length_prefixed(hasher: &mut Hasher, bytes: &[u8]) {
    let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    hasher.update(&length.to_be_bytes());
    hasher.update(bytes);
}
