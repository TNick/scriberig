//! Defines transport-safe trust, validation, and provenance summaries.

use crate::refs::{EntityReference, OperationReference, RevisionReference, SnapshotReference};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Identifies the origin of a semantic value.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum TrustSource {
    /// Written by an author.
    Authored,
    /// Collected from an observed system.
    Observed,
    /// Produced by a model.
    ModelGenerated,
    /// Imported from an external representation.
    Imported,
    /// Produced by the platform itself.
    System,
}

/// Describes validation freshness and outcome.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum ValidationStatus {
    /// Validation is current and passed.
    Current,
    /// Validation is older than the selected state.
    Stale,
    /// Validation completed with a failure.
    Failed,
    /// No validation result is available.
    Unknown,
}

/// Identifies the exact semantic selection being inspected.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct TrustSelection {
    /// Selected entity.
    pub entity: EntityReference,
    /// Exact snapshot context.
    pub snapshot: SnapshotReference,
    /// Optional exact revision context.
    pub revision: Option<RevisionReference>,
}

/// Identifies one immutable provenance input.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProvenanceInput {
    /// Input selection.
    pub selection: TrustSelection,
    /// Semantic relationship to the inspected value.
    pub relation: String,
}

/// Compact trust summary suitable for list and event payloads.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct TrustSummary {
    /// Origin category.
    pub source: TrustSource,
    /// Current validation state.
    pub validation: ValidationStatus,
    /// Optional approval or promotion handle.
    pub accepted_via: Option<String>,
    /// Whether the selected value is restricted.
    pub restricted: bool,
}

/// One safe validation diagnostic.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ValidationDiagnostic {
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Optional semantic location.
    pub location: Option<String>,
}

/// Structured validation result without source or credential payloads.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ValidationSummary {
    /// Overall validation state.
    pub status: ValidationStatus,
    /// Safe diagnostics explaining the result.
    pub diagnostics: Vec<ValidationDiagnostic>,
    /// Optional operation that produced the result.
    pub operation: Option<OperationReference>,
}

/// Safe policy evaluation outcome.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct PolicyEvaluationSummary {
    /// Whether the requested action is allowed.
    pub allowed: bool,
    /// Stable action name.
    pub action: String,
    /// Stable semantic scope.
    pub scope: String,
    /// Safe explanation when denied.
    pub reason: Option<String>,
}

/// Approval or promotion record summary.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ApprovalSummary {
    /// Stable approval or promotion handle.
    pub handle: String,
    /// Whether approval was recorded.
    pub recorded: bool,
    /// Optional operation that recorded it.
    pub operation: Option<OperationReference>,
}

/// Distinguishes promotion outcomes at the protocol boundary.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum PromotionState {
    /// Promotion was accepted.
    Promoted,
    /// Validation rejected promotion.
    Rejected,
    /// The supplied base was stale.
    Stale,
    /// Promotion has a semantic conflict.
    Conflict,
}

/// Result of a candidate promotion attempt.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct PromotionResult {
    /// Promotion outcome.
    pub state: PromotionState,
    /// Candidate snapshot, when one exists.
    pub candidate: Option<SnapshotReference>,
    /// Resulting canonical snapshot, when promoted.
    pub promoted_snapshot: Option<SnapshotReference>,
    /// Safe explanation for a non-promoted result.
    pub reason: Option<String>,
}

/// Summary of a semantic conflict with resolvable handles.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ConflictSummary {
    /// Stable conflict handle.
    pub handle: String,
    /// Entity involved in the conflict.
    pub entity: Option<EntityReference>,
    /// Left side snapshot.
    pub left: Option<SnapshotReference>,
    /// Right side snapshot.
    pub right: Option<SnapshotReference>,
    /// Optional operation requiring resolution.
    pub operation: Option<OperationReference>,
}

/// Known state of an external effect.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum ExternalEffectState {
    /// Effect completed successfully.
    Applied,
    /// Effect is known to have failed.
    Failed,
    /// Effect outcome cannot yet be confirmed.
    Unknown,
}

/// Summary of an external effect receipt.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ExternalEffectSummary {
    /// Stable effect or receipt handle.
    pub handle: String,
    /// Current effect state.
    pub state: ExternalEffectState,
    /// Optional operation that owns the effect.
    pub operation: Option<OperationReference>,
}

/// Structured capability denial safe for presentation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CapabilityDenial {
    /// Requested action.
    pub action: String,
    /// Requested semantic scope.
    pub scope: String,
    /// Safe explanation.
    pub reason: String,
    /// Narrower actions that may be available.
    pub alternatives: Vec<String>,
    /// Whether approval may grant the request.
    pub approval_possible: bool,
}

/// Detailed trust information for one selection.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct TrustDetails {
    /// Compact trust summary.
    pub summary: TrustSummary,
    /// Exact provenance inputs.
    pub provenance: Vec<ProvenanceInput>,
    /// Validation result, when available.
    pub validation: Option<ValidationSummary>,
    /// Optional policy result.
    pub policy: Option<PolicyEvaluationSummary>,
    /// Optional conflict handle.
    pub conflict: Option<ConflictSummary>,
}
