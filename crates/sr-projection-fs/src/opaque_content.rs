//! Binds opaque adapter payloads to exact source evidence and explicit outcomes.

use crate::{FileDigest, digest_bytes};

/// Opaque source bytes retained by an adapter without semantic interpretation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpaquePreservedRecord {
    /// Adapter-defined stable key for this opaque record.
    pub key: String,
    /// Adapter-defined key of the source member that owns this record.
    pub source_member_key: String,
    /// Digest of the exact source member that owns this record.
    pub source_member_digest: FileDigest,
    /// Digest of the bytes retained for round-trip output.
    pub payload_digest: FileDigest,
    /// Exact opaque payload bytes carried through an edit.
    pub payload: Vec<u8>,
    /// Output container digest binding adapter-verified embedded payloads.
    pub output_container_digest: Option<FileDigest>,
}

/// Explicit disposition of opaque content after an adapter operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OpaqueContentOutcome {
    /// The exact opaque payload remains available for output.
    Preserved(OpaquePreservedRecord),
    /// The operation discarded opaque content and records why.
    Discarded {
        /// Adapter-defined key of the removed opaque record.
        key: String,
        /// Adapter-defined key of the source member that contained the record.
        source_member_key: String,
        /// Digest of the exact source member that contained the record.
        source_member_digest: FileDigest,
        /// Provenance for the operation that authorized removal.
        provenance: String,
    },
}

/// Creates an opaque record bound to exact source member and payload bytes.
///
/// # Parameters
/// - `key`: Stable key assigned by the owning adapter.
/// - `source_member_key`: Stable key for the source member that owns the bytes.
/// - `source_member`: Exact source member bytes containing the record.
/// - `payload`: Exact uninterpreted record bytes to preserve.
///
/// # Returns
/// A record whose evidence digests match the provided bytes.
///
/// # Errors
/// Returns an error when the key is empty or the payload is empty.
pub fn preserve_opaque_record(
    key: impl Into<String>,
    source_member_key: impl Into<String>,
    source_member: &[u8],
    payload: &[u8],
) -> Result<OpaquePreservedRecord, OpaqueContentError> {
    let key = key.into();
    let source_member_key = source_member_key.into();
    if key.is_empty() || source_member_key.is_empty() || payload.is_empty() {
        return Err(OpaqueContentError::InvalidRecord);
    }
    Ok(OpaquePreservedRecord {
        key,
        source_member_key,
        source_member_digest: digest_bytes(source_member),
        payload_digest: digest_bytes(payload),
        payload: payload.to_vec(),
        output_container_digest: None,
    })
}

/// Creates an opaque record for bytes embedded inside a rewritten container.
///
/// The adapter must extract the member bytes from both exact containers before
/// calling this function. The output container digest binds that extraction to
/// the exact staged output validated by the shared materializer.
///
/// # Parameters
/// - `key`: Stable key assigned by the owning adapter.
/// - `container_member_key`: Materialization member key of the outer package.
/// - `source_container`: Exact prior container bytes.
/// - `source_payload`: Exact uninterpreted member bytes extracted from source.
/// - `output_container`: Exact staged output container bytes.
/// - `output_payload`: Exact uninterpreted member bytes extracted from output.
///
/// # Returns
/// A record bound to the source container, payload, and staged output digest.
///
/// # Errors
/// Returns an error if evidence keys or payload are empty, the payload changed,
/// or either container is empty.
pub fn preserve_embedded_opaque_record(
    key: impl Into<String>,
    container_member_key: impl Into<String>,
    source_container: &[u8],
    source_payload: &[u8],
    output_container: &[u8],
    output_payload: &[u8],
) -> Result<OpaquePreservedRecord, OpaqueContentError> {
    let key = key.into();
    let source_member_key = container_member_key.into();
    if key.is_empty()
        || source_member_key.is_empty()
        || source_container.is_empty()
        || output_container.is_empty()
    {
        return Err(OpaqueContentError::InvalidRecord);
    }
    if source_payload != output_payload {
        return Err(OpaqueContentError::PayloadChanged);
    }
    Ok(OpaquePreservedRecord {
        key,
        source_member_key,
        source_member_digest: digest_bytes(source_container),
        payload_digest: digest_bytes(source_payload),
        payload: source_payload.to_vec(),
        output_container_digest: Some(digest_bytes(output_container)),
    })
}

/// Confirms an opaque record still matches the bytes prepared for output.
///
/// # Parameters
/// - `record`: Source-bound opaque record to verify.
/// - `output_payload`: Exact bytes the adapter intends to write.
///
/// # Returns
/// Unit when output bytes preserve the exact record payload.
///
/// # Errors
/// Returns an error when the payload differs from its source-bound digest.
pub fn validate_opaque_preservation(
    record: &OpaquePreservedRecord,
    output_payload: &[u8],
) -> Result<(), OpaqueContentError> {
    if digest_bytes(&record.payload) != record.payload_digest
        || digest_bytes(output_payload) != record.payload_digest
    {
        return Err(OpaqueContentError::PayloadChanged);
    }
    Ok(())
}

/// Records deliberate opaque-content removal with operation provenance.
///
/// # Parameters
/// - `key`: Stable key of the record being discarded.
/// - `source_member_key`: Stable key of the source member that held the record.
/// - `source_member_digest`: Digest of the exact source member bytes.
/// - `provenance`: Non-empty operation or policy evidence authorizing removal.
///
/// # Returns
/// An explicit discarded outcome.
///
/// # Errors
/// Returns an error when either value is empty.
pub fn discard_opaque_record(
    key: impl Into<String>,
    source_member_key: impl Into<String>,
    source_member_digest: FileDigest,
    provenance: impl Into<String>,
) -> Result<OpaqueContentOutcome, OpaqueContentError> {
    let key = key.into();
    let source_member_key = source_member_key.into();
    let provenance = provenance.into();
    if key.is_empty() || source_member_key.is_empty() || provenance.is_empty() {
        return Err(OpaqueContentError::MissingDiscardProvenance);
    }
    Ok(OpaqueContentOutcome::Discarded {
        key,
        source_member_key,
        source_member_digest,
        provenance,
    })
}

/// Describes invalid or unsafe opaque-content handling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpaqueContentError {
    /// Opaque records require a stable key and non-empty payload.
    InvalidRecord,
    /// The planned output no longer matches the source-bound payload digest.
    PayloadChanged,
    /// Discarding opaque content requires explicit operation provenance.
    MissingDiscardProvenance,
}
