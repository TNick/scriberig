//! Applies complete project candidate effects through shared filesystem materialization.

use std::path::{Path, PathBuf};

use sr_projection_fs::{
    ArtifactMaterializationMember, ArtifactMaterializationPlan, ArtifactMaterializationReceipt,
    ArtifactMaterializationState, ArtifactMemberReceipt, FileSystem,
    materialize_artifact_validated_with_progress,
};

use super::{EffectPlan, candidate_error};
use crate::source_watch::ProjectSourceWatchCoordinator;
use crate::{ProjectCandidate, ProjectError, ProjectService, ProjectSourceEffectKind};

#[path = "materialize/docx_opaque.rs"]
mod docx_opaque;

/// Executes a complete preflighted set and persists per-path progress.
///
/// - `service`: Project service storing durable progress.
/// - `candidate`: Candidate being materialized.
/// - `plans`: Full preflight result.
/// - `root`: Canonical project root on disk.
/// - `filesystem`: Filesystem adapter used for safe reads and writes.
/// - `source_watch`: Coordinator recording committed output digests.
///
/// # Returns
/// Success after every required effect is materialized, or a structured failure
/// with the exact per-path progress retained.
///
/// # Errors
/// Returns a project error for persistence failure, baseline conflict, or failed
/// filesystem effects.
pub(super) fn materialize_effects(
    service: &ProjectService,
    candidate: &ProjectCandidate,
    plans: &[EffectPlan<'_>],
    root: &Path,
    filesystem: &dyn FileSystem,
    source_watch: &mut ProjectSourceWatchCoordinator,
) -> Result<(), ProjectError> {
    let (materialization, previous) = materialization_plan(service, candidate, plans, root)?;
    let receipt = materialize_artifact_validated_with_progress(
        filesystem,
        &materialization,
        Some(&previous),
        |_| Ok(()),
        |progress| persist_progress(service, candidate, plans, progress, source_watch),
    )
    .map_err(|error| candidate_error("materialization_receipt_failed", &error.to_string()))?;
    if receipt.state == ArtifactMaterializationState::Complete {
        return Ok(());
    }
    Err(materialization_error(&receipt, plans))
}

/// Builds a shared materialization plan and its exact prior progress receipt.
///
/// # Parameters
/// - `service`: Project service loading durable path receipts.
/// - `candidate`: Candidate whose immutable effects are being applied.
/// - `plans`: Complete preflighted effects.
/// - `root`: Authorized project root.
///
/// # Returns
/// Shared executor plan and an attempt receipt bound to this candidate's paths.
///
/// # Errors
/// Returns a storage error when prior path receipts cannot be loaded.
fn materialization_plan(
    service: &ProjectService,
    candidate: &ProjectCandidate,
    plans: &[EffectPlan<'_>],
    root: &Path,
) -> Result<(ArtifactMaterializationPlan, ArtifactMaterializationReceipt), ProjectError> {
    let members = plans
        .iter()
        .map(|plan| ArtifactMaterializationMember {
            key: plan.effect.path.clone(),
            relative_path: PathBuf::from(&plan.effect.path),
            expected_digest: plan
                .effect
                .observed
                .as_deref()
                .or(plan.effect.old.as_deref())
                .map(sr_projection_fs::digest_bytes),
            required: true,
            bytes: plan.effect.new.clone(),
        })
        .collect::<Vec<_>>();
    let opaque_content = docx_opaque::preserved_records(plans)?;
    let materialization = ArtifactMaterializationPlan {
        artifact: candidate.entity,
        root: root.to_path_buf(),
        members: members.clone(),
        opaque_content: opaque_content.clone(),
    };
    let prior_paths = service
        .store
        .project_effect_paths(&candidate.id)
        .map_err(crate::errors::storage_error)?;
    let previous = ArtifactMaterializationReceipt {
        artifact: candidate.entity,
        state: ArtifactMaterializationState::RecoverableFailure,
        members: members
            .iter()
            .map(|member| {
                let durable = prior_paths.iter().find(|record| record.path == member.key);
                ArtifactMemberReceipt {
                    key: member.key.clone(),
                    output_digest: member.bytes.as_deref().map(sr_projection_fs::digest_bytes),
                    observed_digest: durable
                        .and_then(|record| record.observed_digest)
                        .map(sr_projection_fs::FileDigest),
                    committed: durable.is_some_and(|record| {
                        matches!(record.state.as_str(), "applied" | "already_applied")
                    }),
                    failure: durable.and_then(|record| record.diagnostic.clone()),
                }
            })
            .collect(),
        opaque_content,
    };
    Ok((materialization, previous))
}

/// Persists exact progress for each project candidate member.
///
/// # Parameters
/// - `service`: Project service retaining durable per-path state.
/// - `candidate`: Candidate whose effects are being applied.
/// - `plans`: Exact source effects matching receipt member keys.
/// - `progress`: Latest generic materialization receipt.
/// - `source_watch`: Coordinator updated for committed output members.
///
/// # Returns
/// Success after every current member outcome has been written to storage.
///
/// # Errors
/// Returns a storage failure as an IO error so the executor stops before advancing.
fn persist_progress(
    service: &ProjectService,
    candidate: &ProjectCandidate,
    plans: &[EffectPlan<'_>],
    progress: &ArtifactMaterializationReceipt,
    source_watch: &mut ProjectSourceWatchCoordinator,
) -> Result<(), sr_projection_fs::IoError> {
    for outcome in &progress.members {
        let Some(plan) = plans.iter().find(|plan| plan.effect.path == outcome.key) else {
            continue;
        };
        if outcome.committed {
            source_watch.record_committed(
                candidate.project,
                plan.effect.path.clone(),
                outcome.output_digest.map(|digest| digest.0),
            );
        }
        let state = if outcome.committed {
            "applied"
        } else if outcome.failure.is_some() {
            if progress.state == ArtifactMaterializationState::BaselineConflict {
                "conflict"
            } else {
                "failed"
            }
        } else {
            "preflighted"
        };
        service
            .record_effect_path_with_digest(
                candidate,
                plan.effect,
                state,
                if outcome.committed {
                    plan.effect
                        .new
                        .as_deref()
                        .map(|bytes| sr_projection_fs::digest_bytes(bytes).0)
                } else {
                    outcome.observed_digest.map(|digest| digest.0)
                },
                outcome.failure.as_deref(),
            )
            .map_err(|error| {
                sr_projection_fs::IoError::Write(std::io::Error::other(error.message))
            })?;
    }
    Ok(())
}

/// Converts a non-complete executor receipt into the project error expected by callers.
///
/// # Parameters
/// - `receipt`: Shared materialization outcome.
/// - `plans`: Candidate effects used to identify operation-specific failure.
///
/// # Returns
/// A conflict, deletion, or write failure with the executor diagnostic.
fn materialization_error(
    receipt: &ArtifactMaterializationReceipt,
    plans: &[EffectPlan<'_>],
) -> ProjectError {
    let message = receipt
        .members
        .iter()
        .find_map(|member| member.failure.as_deref())
        .unwrap_or("artifact materialization did not complete");
    let failed_kind = receipt
        .members
        .iter()
        .find(|member| member.failure.is_some())
        .and_then(|member| {
            plans
                .iter()
                .find(|plan| plan.effect.path == member.key)
                .map(|plan| plan.effect.kind)
        });
    let code = if receipt.state == ArtifactMaterializationState::BaselineConflict {
        "filesystem_conflict"
    } else if failed_kind == Some(ProjectSourceEffectKind::Delete) {
        "filesystem_delete_failed"
    } else {
        "filesystem_write_failed"
    };
    candidate_error(code, message)
}
