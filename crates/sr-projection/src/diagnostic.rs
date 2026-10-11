//! Defines structured projection lifecycle diagnostics.

use crate::model::{ProjectionRef, SemanticScopeRef};

/// Identifies the projection lifecycle stage associated with a diagnostic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiagnosticStage {
    /// Semantic state could not be rendered.
    Render,
    /// External representation observation failed.
    Observe,
    /// External representation could not be parsed.
    Parse,
    /// Parsed state could not be reconciled.
    Reconcile,
    /// Candidate semantic state failed validation.
    Validate,
    /// Proposed semantic change could not be committed.
    Commit,
    /// Updated canonical state could not be rendered again.
    Rerender,
}

/// Carries machine-readable adapter-specific diagnostic detail.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DiagnosticDetail {
    /// Stable adapter or diagnostic namespace.
    pub code: String,
    /// Structured opaque detail bytes interpreted by the owning adapter.
    pub data: Vec<u8>,
}

/// Describes a failure or concern at one projection lifecycle stage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionDiagnostic {
    /// Lifecycle stage where the diagnostic occurred.
    pub stage: DiagnosticStage,
    /// Projection instance when it is known.
    pub projection: Option<ProjectionRef>,
    /// Semantic scope involved when it is known.
    pub scope: Option<SemanticScopeRef>,
    /// Structured domain-specific diagnostic payload.
    pub detail: DiagnosticDetail,
    /// Optional concise human-readable explanation.
    pub message: Option<String>,
    /// Whether retrying the same operation may succeed.
    pub retryable: bool,
}
