//! Defines explicit candidate promotion evaluation and approval evidence.

use crate::validation::{ValidationEvidence, evidence_applicable};
use crate::{Capability, CapabilityAction, CapabilityScope};
use scriberig_core::{EntityRevisionRef, PrincipalId, RootName, SnapshotId};
use sr_change::{PromotionResult, promote};

/// Structured decision returned by a promotion policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromotionDecision {
    /// Candidate may advance.
    Allow,
    /// Candidate is rejected.
    Reject,
    /// Candidate needs approval.
    RequireApproval,
    /// Candidate needs more validation.
    RequireAdditionalValidation,
    /// Canonical state changed.
    ConflictOrStale,
}

/// Attributable policy evaluation for one candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionPolicyEvaluation {
    /// Structured decision.
    pub decision: PromotionDecision,
    /// Policy identity and revision.
    pub policy: String,
}

/// Approval evidence bound to one exact candidate and policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalEvidence {
    /// Approving principal.
    pub principal: PrincipalId,
    /// Exact candidate snapshot.
    pub candidate: SnapshotId,
    /// Policy context approved.
    pub policy: String,
    /// Stable operation/evidence handle.
    pub operation: String,
}

/// Inputs required to evaluate promotion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionRequest {
    /// Principal requesting promotion.
    pub principal: PrincipalId,
    /// Canonical snapshot expected by the proposer.
    pub expected_base: SnapshotId,
    /// Candidate snapshot to promote.
    pub candidate: SnapshotId,
    /// Target canonical root.
    pub root: RootName,
    /// Exact candidate dependencies.
    pub dependencies: Option<Vec<EntityRevisionRef>>,
    /// Validation evidence available to the evaluator.
    pub evidence: Vec<ValidationEvidence>,
    /// Requested policy identity.
    pub policy: String,
    /// Optional attributable approval.
    pub approval: Option<ApprovalEvidence>,
}

/// Evaluates capability, freshness, evidence, policy, and approval in order.
///
/// # Parameters
/// - `request`: Explicit promotion request.
/// - `current_root`: Current canonical root snapshot.
/// - `capability`: Capability presented by the principal.
/// - `policy`: Attributable policy evaluation.
///
/// # Returns
/// The structured decision for the candidate.
#[must_use]
pub fn evaluate_promotion(
    request: &PromotionRequest,
    current_root: SnapshotId,
    capability: &Capability,
    policy: &PromotionPolicyEvaluation,
) -> PromotionDecision {
    let scope = CapabilityScope::Branch(request.root.clone());
    if !capability.covers(
        &CapabilityAction::Promote,
        &scope,
        Some(request.expected_base),
        None,
    ) {
        return PromotionDecision::Reject;
    }
    if current_root != request.expected_base {
        return PromotionDecision::ConflictOrStale;
    }
    let valid = request.evidence.iter().any(|evidence| {
        evidence.status == crate::ValidationStatus::Passed
            && evidence_applicable(
                evidence,
                Some(request.candidate),
                request.dependencies.as_deref(),
            )
    });
    if !valid {
        return PromotionDecision::RequireAdditionalValidation;
    }
    match policy.decision {
        PromotionDecision::RequireApproval
            if request.approval.as_ref().is_none_or(|approval| {
                approval.candidate != request.candidate || approval.policy != request.policy
            }) =>
        {
            PromotionDecision::RequireApproval
        }
        decision => decision,
    }
}

/// Applies a previously evaluated promotion through atomic root CAS.
pub fn promote_candidate<S: scriberig_store::RootStore + scriberig_store::SnapshotStore>(
    store: &S,
    request: &PromotionRequest,
    decision: PromotionDecision,
) -> PromotionResult {
    if decision != PromotionDecision::Allow {
        return PromotionResult::InvalidTarget;
    }
    promote(
        store,
        request.root.clone(),
        Some(request.expected_base),
        request.candidate,
    )
}
