//! Validates and decodes shared semantic plan records.

use std::collections::BTreeMap;

use scriberig_core::{EncodingVersion, revision_id};
use serde::Deserialize;

use super::super::PlanSemanticRevision;
use super::super::encode::{HISTORY_TYPE, relation_id};
use super::super::{PlanSemanticError, PlanSemanticHistory, PlanSemanticSnapshot};
use super::payloads::{HistoryIdentity, HistoryPayload};

/// Decodes safe mutation metadata from a semantic plan object set.
///
/// - `snapshot`: Semantic plan records to inspect.
///
/// Ordered revision metadata.
///
/// Returns an error for malformed or mismatched history objects.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(in crate::planning::semantic) fn history_metadata(
    snapshot: &PlanSemanticSnapshot,
) -> Result<Vec<PlanSemanticHistory>, PlanSemanticError> {
    let mut metadata = BTreeMap::new();
    for object in &snapshot.objects {
        if object.object_type.as_str() != HISTORY_TYPE {
            continue;
        }
        validate_revision(object)?;
        let payload: HistoryPayload = decode_payload(object)?;
        if payload.schema_version != 1
            || payload.plan != snapshot.plan
            || relation_id(
                HISTORY_TYPE,
                &HistoryIdentity {
                    plan: payload.plan,
                    mutation_revision: payload.mutation_revision,
                },
            )? != object.entity
            || metadata
                .insert(payload.mutation_revision, payload.reason)
                .is_some()
        {
            return Err(invalid(
                "duplicate, mismatched, or unsupported history record",
            ));
        }
    }
    Ok(metadata
        .into_iter()
        .map(|(mutation_revision, reason)| PlanSemanticHistory {
            mutation_revision,
            reason,
        })
        .collect())
}

/// Verifies the canonical content identity before decoding an object.
///
/// - `object`: Semantic object to verify.
///
/// Unit when the payload hashes to its recorded immutable revision.
///
/// Returns `InvalidObject` when the revision does not match its payload.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn validate_revision(object: &PlanSemanticRevision) -> Result<(), PlanSemanticError> {
    let expected = revision_id(EncodingVersion::V1, &object.object_type, &object.payload)
        .map_err(|error| PlanSemanticError::Serialization(format!("{error:?}")))?;
    if expected != object.revision {
        return Err(invalid("object revision does not match canonical payload"));
    }
    Ok(())
}

/// Decodes one typed JSON payload from an immutable object.
///
/// - `object`: Revision whose payload is decoded.
///
/// The versioned typed payload.
///
/// Returns a serialization error when bytes are invalid.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn decode_payload<T: for<'de> Deserialize<'de>>(
    object: &PlanSemanticRevision,
) -> Result<T, PlanSemanticError> {
    serde_json::from_slice(&object.payload)
        .map_err(|error| PlanSemanticError::Serialization(error.to_string()))
}

/// Creates a typed malformed-object diagnostic.
///
/// - `message`: Bounded explanation of the malformed object.
///
/// A semantic plan decoding error.
/// # Returns
/// The operation result.
pub(super) fn invalid(message: impl Into<String>) -> PlanSemanticError {
    PlanSemanticError::InvalidObject(message.into())
}
