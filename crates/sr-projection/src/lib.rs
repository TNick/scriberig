//! Defines domain-independent projection contracts and lifecycle types.

// Keep projection concepts separated by contract responsibility.
pub mod adapter;
pub mod capabilities;
pub mod conflict;
pub mod diagnostic;
pub mod fidelity;
pub mod model;
pub mod purpose;
pub mod reconcile;
pub mod reconciliation_baseline;
pub mod render;
pub mod state;
pub mod style;
pub mod style_adapter;
pub mod style_schema;
pub mod style_source;

// Expose the principal shared identifiers and contracts.
pub use adapter::{ProjectionAdapter, ProjectionResult, ReconciliationContext};
pub use capabilities::{ProjectionCapabilities, ProjectionFeature};
pub use conflict::{
    AmbiguousMapping, CapturedObservation, ConflictId, ConflictKind, ConflictResolution,
    ConflictStatus, ObservationId, ProjectionConflict, ProjectionConflictStore, ResolvedConflict,
};
pub use diagnostic::{
    DiagnosticDetail, DiagnosticKind, DiagnosticSource, DiagnosticStage, ProjectionDiagnostic,
};
pub use fidelity::{FidelityAssessment, ProjectionFidelity};
pub use model::{
    ExternalLocation, OwnershipClass, ProjectionClass, ProjectionKind, ProjectionRef,
    ProjectionSpec, ProjectionSpecRef, RendererVersion, SemanticScopeRef, SemanticStateRef,
};
pub use purpose::ProjectionPurpose;
pub use reconcile::ReconciliationOutcome;
pub use reconciliation_baseline::{
    ProjectionAdapterIdentity, ProjectionEvidence, ReconciliationBaseline,
};
pub use render::{
    OutputHash, RenderCacheKey, RenderRequest, RenderedFragment, RenderedProjection,
    RenderingContext,
};
pub use state::ProjectionStatus;
pub use style::{ResolvedStyle, StyleRef, StyleResolutionError, StyleSchemaRef, StyleValue};
pub use style_adapter::{
    ProjectionStyleAdapter, invalid_style_diagnostic, unsupported_style_diagnostic,
};
pub use style_schema::{
    StyleApplicability, StylePropertyDescriptor, StyleSchema, StyleSchemaError, StyleValueKind,
};
pub use style_source::{
    ResolvedStyleProvenance, StylePropertyOutcome, StyleSource, StyleSourceKind,
};
