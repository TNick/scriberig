//! Defines domain-independent projection contracts and lifecycle types.

// Keep projection concepts separated by contract responsibility.
pub mod adapter;
pub mod capabilities;
pub mod conflict;
pub mod diagnostic;
pub mod model;
pub mod reconcile;
pub mod render;
pub mod state;
pub mod style;

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
pub use model::{
    ExternalLocation, OwnershipClass, ProjectionClass, ProjectionKind, ProjectionRef,
    ProjectionSpec, ProjectionSpecRef, RendererVersion, SemanticScopeRef, SemanticStateRef,
};
pub use reconcile::ReconciliationOutcome;
pub use render::{
    OutputHash, RenderCacheKey, RenderRequest, RenderedFragment, RenderedProjection,
    RenderingContext,
};
pub use state::ProjectionStatus;
pub use style::{StyleRef, StyleValue};
