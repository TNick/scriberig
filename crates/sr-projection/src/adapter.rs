//! Defines the domain adapter boundary for projection lifecycle operations.

use crate::{
    capabilities::ProjectionCapabilities,
    diagnostic::ProjectionDiagnostic,
    model::ProjectionKind,
    reconcile::ReconciliationOutcome,
    render::{RenderRequest, RenderedProjection},
};

/// Returns projection results with heap-allocated diagnostics.
pub type ProjectionResult<Value> = Result<Value, Box<ProjectionDiagnostic>>;

/// Connects one domain representation to generic projection operations.
///
/// # Public members
/// - `capabilities`: Declares supported operations.
/// - `render`: Produces an external representation.
/// - `parse`: Parses an external representation into adapter-owned candidate data.
/// - `reconcile`: Relates parsed data to exact canonical state.
/// - `validate`: Checks adapter-specific candidate invariants.
///
/// # Private members
/// - None.
pub trait ProjectionAdapter {
    /// Adapter-owned parsed semantic candidate.
    type Candidate;
    /// Adapter-owned proposed semantic migration.
    type Migration;
    /// Adapter-owned semantic conflict representation.
    type Conflict;

    /// Returns capabilities for the requested projection kind.
    ///
    /// # Parameters
    /// - `self`: Adapter providing the contract.
    /// - `kind`: Projection kind to inspect.
    ///
    /// # Returns
    /// Capabilities declared for the kind.
    fn capabilities(&self, kind: &ProjectionKind) -> ProjectionCapabilities;

    /// Renders semantic state using the explicit request inputs.
    ///
    /// # Parameters
    /// - `self`: Adapter performing rendering.
    /// - `request`: Exact render inputs.
    ///
    /// # Returns
    /// The rendered representation.
    ///
    /// # Errors
    /// Returns a structured diagnostic when rendering fails.
    fn render(&self, request: &RenderRequest) -> ProjectionResult<RenderedProjection>;

    /// Parses an external representation without assigning canonical identity.
    ///
    /// # Parameters
    /// - `self`: Adapter performing parsing.
    /// - `kind`: External representation format.
    /// - `bytes`: Exact external representation bytes.
    ///
    /// # Returns
    /// An adapter-owned semantic candidate.
    ///
    /// # Errors
    /// Returns a structured diagnostic when parsing fails.
    fn parse(&self, kind: &ProjectionKind, bytes: &[u8]) -> ProjectionResult<Self::Candidate>;

    /// Reconciles a candidate against the exact state from which it was edited.
    ///
    /// # Parameters
    /// - `self`: Adapter performing reconciliation.
    /// - `candidate`: Parsed external candidate.
    /// - `base`: Exact canonical base snapshot.
    ///
    /// # Returns
    /// A typed outcome distinguishing no-op, proposal, ambiguity, conflict,
    /// and invalid external state.
    fn reconcile(
        &self,
        candidate: Self::Candidate,
        base: crate::model::SemanticStateRef,
    ) -> ReconciliationOutcome<Self::Migration, Self::Conflict, Box<ProjectionDiagnostic>>;

    /// Validates an adapter-owned candidate before proposing canonical change.
    ///
    /// # Parameters
    /// - `self`: Adapter performing validation.
    /// - `candidate`: Parsed external candidate.
    ///
    /// # Returns
    /// Success when the candidate satisfies adapter invariants.
    ///
    /// # Errors
    /// Returns a structured diagnostic when validation fails.
    fn validate(&self, candidate: &Self::Candidate) -> ProjectionResult<()>;
}
