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
    /// A projection style could not be resolved, normalized, or merged.
    StyleResolution,
}

/// Classifies a projection diagnostic independently of presentation text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiagnosticKind {
    /// The requested adapter operation is unsupported.
    UnsupportedOperation,
    /// Projection configuration is invalid.
    InvalidConfiguration,
    /// Rendering failed.
    RenderFailure,
    /// Parsing failed.
    ParseFailure,
    /// Reconciliation found ambiguity.
    ReconciliationAmbiguous,
    /// Reconciliation found a conflict.
    ReconciliationConflict,
    /// Candidate validation failed.
    ValidationFailure,
    /// The synchronization base is missing or stale.
    SynchronizationBaseMissing,
    /// A storage or semantic-change operation failed.
    StorageFailure,
}

/// Carries machine-readable adapter-specific diagnostic detail.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DiagnosticDetail {
    /// Stable adapter or diagnostic namespace.
    pub code: String,
    /// Structured opaque detail bytes interpreted by the owning adapter.
    pub data: Vec<u8>,
}

/// Preserves the structured lower-level source of a projection diagnostic.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DiagnosticSource {
    /// Subsystem or adapter that produced the underlying failure.
    pub subsystem: String,
    /// Stable source-specific error code.
    pub code: String,
    /// Source-provided explanation when available.
    pub message: Option<String>,
}

/// Describes a failure or concern at one projection lifecycle stage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionDiagnostic {
    /// Stable diagnostic category.
    pub kind: DiagnosticKind,
    /// Lifecycle stage where the diagnostic occurred.
    pub stage: DiagnosticStage,
    /// Projection instance when it is known.
    pub projection: Option<ProjectionRef>,
    /// Semantic scope involved when it is known.
    pub scope: Option<SemanticScopeRef>,
    /// Structured domain-specific diagnostic payload.
    pub detail: DiagnosticDetail,
    /// Structured lower-level source of the diagnostic, when available.
    pub source: Option<DiagnosticSource>,
    /// Optional concise human-readable explanation.
    pub message: Option<String>,
    /// Whether retrying the same operation may succeed.
    pub retryable: bool,
}

impl ProjectionDiagnostic {
    /// Creates a structured diagnostic for an unsupported adapter operation.
    ///
    /// - `stage`: Lifecycle stage where the unsupported operation was requested.
    ///
    /// An explicit non-retryable unsupported-operation diagnostic.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn unsupported(stage: DiagnosticStage) -> Self {
        Self {
            kind: DiagnosticKind::UnsupportedOperation,
            stage,
            projection: None,
            scope: None,
            detail: DiagnosticDetail {
                code: "unsupported-operation".to_owned(),
                data: Vec::new(),
            },
            source: Some(DiagnosticSource {
                subsystem: "projection-adapter".to_owned(),
                code: "unsupported-operation".to_owned(),
                message: None,
            }),
            message: Some("the projection adapter does not support this operation".to_owned()),
            retryable: false,
        }
    }
}
