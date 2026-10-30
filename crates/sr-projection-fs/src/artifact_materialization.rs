//! Stages, commits, and recovers multi-member artifact materialization.

use std::path::PathBuf;

use scriberig_core::EntityId;

use crate::{FileDigest, FileSystem, IoError, OpaqueContentOutcome, digest_bytes};

mod validation;
use validation::validate_plan;

/// A complete artifact write prepared before any target member is replaced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactMaterializationPlan {
    /// Stable semantic artifact entity.
    pub artifact: EntityId,
    /// Authorized project root for all relative member paths.
    pub root: PathBuf,
    /// Required and optional member outputs, where `None` deletes a member.
    pub members: Vec<ArtifactMaterializationMember>,
    /// Opaque records that must survive or be explicitly discarded.
    pub opaque_content: Vec<OpaqueContentOutcome>,
}

/// Exact replacement data and baseline for one artifact member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactMaterializationMember {
    /// Adapter-owned member key.
    pub key: String,
    /// Safe project-relative output path.
    pub relative_path: PathBuf,
    /// Expected old digest; None means the member must not exist.
    pub expected_digest: Option<FileDigest>,
    /// Whether failure to commit this output makes the artifact incomplete.
    pub required: bool,
    /// Fully rendered bytes, or `None` when the member is deleted.
    pub bytes: Option<Vec<u8>>,
}

/// Records exact per-member outcomes for an artifact write or recovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactMaterializationReceipt {
    /// Stable semantic artifact entity.
    pub artifact: EntityId,
    /// State after the latest execution attempt.
    pub state: ArtifactMaterializationState,
    /// Member outcomes ordered as they appear in the plan.
    pub members: Vec<ArtifactMemberReceipt>,
    /// Opaque content preservation/discard decisions applied to this artifact.
    pub opaque_content: Vec<OpaqueContentOutcome>,
}

/// States whether a multi-member artifact write is complete or recoverable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactMaterializationState {
    /// No member replacement has been attempted.
    Pending,
    /// Every required member was committed.
    Complete,
    /// At least one required member failed after the attempt began.
    RecoverableFailure,
    /// An external change prevented the operation from starting or recovering.
    BaselineConflict,
    /// Candidate validation rejected staged output before filesystem writes.
    ValidationFailed,
}

/// Records a member digest and whether replacement completed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactMemberReceipt {
    /// Adapter-owned member key.
    pub key: String,
    /// Digest of the exact planned output, or `None` when the member is deleted.
    pub output_digest: Option<FileDigest>,
    /// Digest observed at the target when it differed from the expected baseline.
    pub observed_digest: Option<FileDigest>,
    /// Whether the target now matches the planned output state.
    pub committed: bool,
    /// Safe failure description when replacement failed.
    pub failure: Option<String>,
}

/// Executes a fully rendered artifact plan with baseline checks and recovery.
///
/// Every target is checked before the first replacement. A prior receipt makes
/// retries idempotent when already committed members still match their output
/// digests. The returned receipt must be durably retained by the caller.
///
/// - filesystem: Filesystem boundary used for reads and atomic member writes.
/// - plan: Complete plan with all rendered member bytes.
/// - previous: Receipt from an earlier attempt, when recovering.
///
/// Exact per-member result and recovery state.
///
/// Returns an IO or invalid-plan error during preflight; conflicts and failures
/// after the attempt begins are recorded in the returned receipt.
pub fn materialize_artifact<F: FileSystem + ?Sized>(
    filesystem: &F,
    plan: &ArtifactMaterializationPlan,
    previous: Option<&ArtifactMaterializationReceipt>,
) -> Result<ArtifactMaterializationReceipt, IoError> {
    materialize_artifact_validated(filesystem, plan, previous, |_| Ok(()))
}

/// Validates the complete rendered plan before committing any artifact member.
///
/// - filesystem: Filesystem boundary used for reads and atomic member writes.
/// - plan: Complete plan with all rendered member bytes.
/// - previous: Receipt from an earlier attempt, when recovering.
/// - validate: Candidate validator over the complete immutable output plan.
///
/// Exact per-member result and recovery state.
///
/// Returns an IO or invalid-plan error during preflight; validator rejection is
/// represented by a `ValidationFailed` receipt and performs no filesystem IO.
pub fn materialize_artifact_validated<F, V>(
    filesystem: &F,
    plan: &ArtifactMaterializationPlan,
    previous: Option<&ArtifactMaterializationReceipt>,
    validate: V,
) -> Result<ArtifactMaterializationReceipt, IoError>
where
    F: FileSystem + ?Sized,
    V: FnOnce(&ArtifactMaterializationPlan) -> Result<(), String>,
{
    materialize_artifact_validated_with_progress(filesystem, plan, previous, validate, |_| Ok(()))
}

/// Materializes a plan while reporting durable progress after each member outcome.
///
/// - `filesystem`: Filesystem boundary for reads and atomic writes.
/// - `plan`: Complete immutable artifact plan.
/// - `previous`: Prior attempt receipt used for idempotent recovery.
/// - `validate`: Validator run against the complete plan before filesystem effects.
/// - `progress`: Callback that persists the latest exact receipt after each transition.
///
/// # Returns
/// The final per-member outcome receipt.
///
/// # Errors
/// Returns filesystem or progress-persistence failures.
pub fn materialize_artifact_validated_with_progress<F, V, P>(
    filesystem: &F,
    plan: &ArtifactMaterializationPlan,
    previous: Option<&ArtifactMaterializationReceipt>,
    validate: V,
    mut progress: P,
) -> Result<ArtifactMaterializationReceipt, IoError>
where
    F: FileSystem + ?Sized,
    V: FnOnce(&ArtifactMaterializationPlan) -> Result<(), String>,
    P: FnMut(&ArtifactMaterializationReceipt) -> Result<(), IoError>,
{
    validate_plan(plan)?;
    if let Err(error) = validate(plan) {
        let receipt = ArtifactMaterializationReceipt {
            artifact: plan.artifact,
            state: ArtifactMaterializationState::ValidationFailed,
            members: plan
                .members
                .iter()
                .map(|member| ArtifactMemberReceipt {
                    key: member.key.clone(),
                    output_digest: member.bytes.as_deref().map(digest_bytes),
                    observed_digest: None,
                    committed: false,
                    failure: Some(format!("candidate validation failed: {error}")),
                })
                .collect(),
            opaque_content: plan.opaque_content.clone(),
        };
        progress(&receipt)?;
        return Ok(receipt);
    }
    let mut committed_before = Vec::with_capacity(plan.members.len());
    let mut observed_before = Vec::with_capacity(plan.members.len());
    let mut baseline_conflict = false;
    for member in &plan.members {
        let path = plan.root.join(&member.relative_path);
        let current = filesystem
            .metadata(&path)?
            .map(|_| filesystem.read(&path))
            .transpose()?;
        let current_digest = current.as_deref().map(digest_bytes);
        let was_committed = previous
            .filter(|receipt| receipt.artifact == plan.artifact)
            .and_then(|receipt| receipt.members.iter().find(|item| item.key == member.key))
            .is_some_and(|prior| {
                prior.output_digest == member.bytes.as_deref().map(digest_bytes)
                    && current_digest == member.bytes.as_deref().map(digest_bytes)
            });
        if !was_committed && current_digest != member.expected_digest {
            baseline_conflict = true;
        }
        committed_before.push(was_committed);
        observed_before.push(current_digest);
    }

    let mut receipt = ArtifactMaterializationReceipt {
        artifact: plan.artifact,
        state: ArtifactMaterializationState::Pending,
        members: plan
            .members
            .iter()
            .zip(&committed_before)
            .zip(&observed_before)
            .map(|((member, committed), observed)| ArtifactMemberReceipt {
                key: member.key.clone(),
                output_digest: member.bytes.as_deref().map(digest_bytes),
                observed_digest: *observed,
                committed: *committed,
                failure: None,
            })
            .collect(),
        opaque_content: plan.opaque_content.clone(),
    };
    if baseline_conflict {
        receipt.state = ArtifactMaterializationState::BaselineConflict;
        for (index, _) in plan.members.iter().enumerate() {
            if !committed_before[index] {
                receipt.members[index].failure = Some("external baseline changed".into());
            }
        }
        progress(&receipt)?;
        return Ok(receipt);
    }
    progress(&receipt)?;
    commit_unwritten_members(
        filesystem,
        plan,
        &committed_before,
        &mut receipt,
        &mut progress,
    )?;
    progress(&receipt)?;
    Ok(receipt)
}

/// Rechecks and atomically writes each member not already committed.
///
/// # Parameters
/// - `filesystem`: Filesystem boundary for baseline checks and writes.
/// - `plan`: Complete rendered artifact output plan.
/// - `committed_before`: Members already matching a prior receipt.
/// - `receipt`: Mutable record of each member outcome.
fn commit_unwritten_members<F: FileSystem + ?Sized>(
    filesystem: &F,
    plan: &ArtifactMaterializationPlan,
    committed_before: &[bool],
    receipt: &mut ArtifactMaterializationReceipt,
    progress: &mut impl FnMut(&ArtifactMaterializationReceipt) -> Result<(), IoError>,
) -> Result<(), IoError> {
    for (index, member) in plan.members.iter().enumerate() {
        if committed_before[index] {
            continue;
        }
        let path = plan.root.join(&member.relative_path);
        let current = match filesystem.metadata(&path) {
            Ok(Some(_)) => match filesystem.read(&path) {
                Ok(bytes) => Some(bytes),
                Err(error) => {
                    record_member_failure(receipt, index, member.required, error.to_string());
                    progress(receipt)?;
                    continue;
                }
            },
            Ok(None) => None,
            Err(error) => {
                record_member_failure(receipt, index, member.required, error.to_string());
                progress(receipt)?;
                continue;
            }
        };
        if current.as_deref().map(digest_bytes) != member.expected_digest {
            receipt.state = ArtifactMaterializationState::BaselineConflict;
            receipt.members[index].observed_digest = current.as_deref().map(digest_bytes);
            receipt.members[index].failure = Some("external baseline changed during commit".into());
            progress(receipt)?;
            return Ok(());
        }
        if let Some(parent) = path.parent()
            && let Err(error) = filesystem.create_dir_all(parent)
        {
            record_member_failure(receipt, index, member.required, error.to_string());
            progress(receipt)?;
            continue;
        }
        let write_result = if let Some(bytes) = &member.bytes {
            filesystem.write_atomic(&path, bytes)
        } else {
            filesystem.remove_file(&path)
        };
        if let Err(error) = write_result {
            record_member_failure(receipt, index, member.required, error.to_string());
            progress(receipt)?;
            continue;
        }
        receipt.members[index].committed = true;
        progress(receipt)?;
    }
    if receipt.state != ArtifactMaterializationState::RecoverableFailure {
        receipt.state = ArtifactMaterializationState::Complete;
    }
    Ok(())
}

/// Records one member failure and upgrades required failures to recoverable.
///
/// # Parameters
/// - `receipt`: Attempt receipt to update.
/// - `index`: Member receipt index to update.
/// - `required`: Whether this member is required for artifact completeness.
/// - `failure`: Safe failure description.
fn record_member_failure(
    receipt: &mut ArtifactMaterializationReceipt,
    index: usize,
    required: bool,
    failure: String,
) {
    receipt.members[index].failure = Some(failure);
    if required {
        receipt.state = ArtifactMaterializationState::RecoverableFailure;
    }
}
