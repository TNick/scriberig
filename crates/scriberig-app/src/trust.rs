//! Defines trust and provenance summaries shared by application surfaces.

use scriberig_core::{RevisionId, SnapshotId};

/// Classifies how an inspected object entered the knowledge system.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceKind {
    /// Content authored directly by a project actor.
    Authored,
    /// Content observed from an external source.
    Observed,
    /// Content imported from an external representation.
    Imported,
    /// Content produced by deterministic parsing or transformation.
    Derived,
    /// Content produced by a model or other interpretive process.
    ModelGenerated,
}

/// Describes the freshness and outcome of validation evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationState {
    /// Evidence was produced for the exact inspected revision.
    Current,
    /// Evidence exists but targets an older revision or snapshot.
    Stale,
    /// Validation produced a failure.
    Failed,
    /// No applicable validation evidence is available.
    Unknown,
}

/// Summarizes trust-relevant metadata without copying object content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustSummary {
    /// How the object entered the system.
    pub source: SourceKind,
    /// Freshness and outcome of validation evidence.
    pub validation: ValidationState,
    /// Handle for the migration or promotion that accepted the object.
    pub accepted_via: Option<String>,
    /// Handles for deeper provenance inspection.
    pub provenance: Vec<String>,
    /// Exact snapshot used for this summary when known.
    pub snapshot: Option<SnapshotId>,
    /// Exact revision summarized when known.
    pub revision: Option<RevisionId>,
}

/// Describes a capability denial without exposing restricted content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityDenial {
    /// Action the caller requested.
    pub action: String,
    /// Scope against which the action was requested.
    pub scope: String,
    /// Policy reason for the denial.
    pub reason: String,
    /// Narrower actions that remain available.
    pub alternatives: Vec<String>,
    /// Whether an approval could grant the requested action.
    pub approval_may_grant: bool,
}
