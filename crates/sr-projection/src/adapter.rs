//! Defines the domain adapter boundary for projection lifecycle operations.

use crate::{
    capabilities::ProjectionCapabilities,
    diagnostic::ProjectionDiagnostic,
    model::ProjectionKind,
    reconcile::ReconciliationOutcome,
    render::{RenderRequest, RenderedProjection},
};

/// Groups exact state and specification inputs needed for reconciliation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationContext {
    /// Exact synchronized output and the immutable inputs that produced it.
    pub baseline: crate::ReconciliationBaseline,
    /// Current canonical state.
    pub current: crate::model::SemanticStateRef,
}

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
    /// - `self`: Adapter providing the contract.
    /// - `kind`: Projection kind to inspect.
    ///
    /// Capabilities declared for the kind.
    /// # Returns
    /// The operation result.
    fn capabilities(&self, kind: &ProjectionKind) -> ProjectionCapabilities;

    /// Renders semantic state using the explicit request inputs.
    ///
    /// - `self`: Adapter performing rendering.
    /// - `request`: Exact render inputs.
    ///
    /// The rendered representation.
    ///
    /// Returns a structured diagnostic when rendering fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn render(&self, request: &RenderRequest) -> ProjectionResult<RenderedProjection>;

    /// Parses an external representation without assigning canonical identity.
    ///
    /// - `self`: Adapter performing parsing.
    /// - `kind`: External representation format.
    /// - `bytes`: Exact external representation bytes.
    ///
    /// An adapter-owned semantic candidate.
    ///
    /// Returns a structured diagnostic when parsing fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn parse(&self, kind: &ProjectionKind, bytes: &[u8]) -> ProjectionResult<Self::Candidate> {
        let _ = (kind, bytes);
        Err(Box::new(ProjectionDiagnostic::unsupported(
            crate::diagnostic::DiagnosticStage::Parse,
        )))
    }

    /// Reconciles a candidate against the exact state from which it was edited.
    ///
    /// - `self`: Adapter performing reconciliation.
    /// - `candidate`: Parsed external candidate.
    /// - `context`: Exact canonical and projection synchronization context.
    ///
    /// A typed outcome distinguishing no-op, proposal, ambiguity, conflict,
    /// and invalid external state.
    /// # Returns
    /// The operation result.
    fn reconcile(
        &self,
        candidate: Self::Candidate,
        context: &ReconciliationContext,
    ) -> ReconciliationOutcome<Self::Migration, Self::Conflict, Box<ProjectionDiagnostic>> {
        let _ = (candidate, context);
        ReconciliationOutcome::InvalidExternalState(Box::new(ProjectionDiagnostic::unsupported(
            crate::diagnostic::DiagnosticStage::Reconcile,
        )))
    }

    /// Validates an adapter-owned candidate before proposing canonical change.
    ///
    /// - `self`: Adapter performing validation.
    /// - `candidate`: Parsed external candidate.
    ///
    /// Success when the candidate satisfies adapter invariants.
    ///
    /// Returns a structured diagnostic when validation fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn validate(&self, candidate: &Self::Candidate) -> ProjectionResult<()> {
        let _ = candidate;
        Err(Box::new(ProjectionDiagnostic::unsupported(
            crate::diagnostic::DiagnosticStage::Validate,
        )))
    }
}
