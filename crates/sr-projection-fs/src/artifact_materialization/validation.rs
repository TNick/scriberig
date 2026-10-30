//! Validates materialization plans before any filesystem operation begins.

use std::path::{Component, Path};

use crate::{IoError, OpaqueContentOutcome, validate_opaque_preservation};

use super::ArtifactMaterializationPlan;

/// Validates all member paths, keys, and opaque preservation outcomes.
///
/// - `plan`: Artifact plan to validate.
///
/// # Returns
/// Success when every member target and preservation requirement is valid.
///
/// # Errors
/// Returns an unsafe-plan error for empty plans, duplicate identities/paths,
/// unsafe paths, or invalid opaque-content treatment.
pub(super) fn validate_plan(plan: &ArtifactMaterializationPlan) -> Result<(), IoError> {
    if plan.members.is_empty() {
        return Err(IoError::UnsafeArtifactPlan);
    }
    for (index, member) in plan.members.iter().enumerate() {
        if member.key.is_empty()
            || plan.members[..index]
                .iter()
                .any(|other| other.key == member.key || other.relative_path == member.relative_path)
            || !safe_relative_path(&member.relative_path)
        {
            return Err(IoError::UnsafeArtifactPlan);
        }
    }
    for outcome in &plan.opaque_content {
        let (member_key, source_digest) = match outcome {
            OpaqueContentOutcome::Preserved(record) => {
                if validate_opaque_preservation(record, &record.payload).is_err() {
                    return Err(IoError::UnsafeArtifactPlan);
                }
                (&record.source_member_key, record.source_member_digest)
            }
            OpaqueContentOutcome::Discarded {
                source_member_key,
                source_member_digest,
                provenance,
                ..
            } => {
                if provenance.is_empty() {
                    return Err(IoError::UnsafeArtifactPlan);
                }
                (source_member_key, *source_member_digest)
            }
        };
        let Some(member) = plan.members.iter().find(|member| &member.key == member_key) else {
            return Err(IoError::UnsafeArtifactPlan);
        };
        let output_matches = match outcome {
            OpaqueContentOutcome::Preserved(record) => member.bytes.as_ref().is_some_and(|bytes| {
                record.output_container_digest.map_or_else(
                    || {
                        bytes
                            .windows(record.payload.len())
                            .any(|window| window == record.payload)
                    },
                    |digest| crate::digest_bytes(bytes) == digest,
                )
            }),
            OpaqueContentOutcome::Discarded { .. } => true,
        };
        if member.expected_digest != Some(source_digest) || !output_matches {
            return Err(IoError::UnsafeArtifactPlan);
        }
    }
    Ok(())
}

/// Reports whether a path is a non-empty relative path without traversal.
///
/// - `path`: Member path to validate.
///
/// # Returns
/// True when every component is a normal relative path component with no traversal.
#[must_use]
fn safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && !path.is_absolute()
}
