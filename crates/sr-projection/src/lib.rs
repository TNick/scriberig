//! Defines domain-independent projection contracts and lifecycle types.

// Keep projection concepts separated by contract responsibility.
pub mod adapter;
pub mod capabilities;
pub mod diagnostic;
pub mod model;
pub mod reconcile;
pub mod render;
pub mod state;
pub mod style;

// Expose the principal shared identifiers and contracts.
pub use adapter::ProjectionAdapter;
pub use capabilities::ProjectionCapabilities;
pub use diagnostic::{DiagnosticDetail, DiagnosticStage, ProjectionDiagnostic};
pub use model::{
    OwnershipClass, ProjectionClass, ProjectionKind, ProjectionRef, ProjectionSpec,
    ProjectionSpecRef, SemanticScopeRef,
};
pub use reconcile::ReconciliationOutcome;
pub use render::{RenderRequest, RenderedProjection};
pub use state::ProjectionStatus;
pub use style::{StyleRef, StyleValue};
