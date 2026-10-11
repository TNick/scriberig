//! Renders shared trust summaries for human-readable CLI inspection.

use std::fmt::Write;

use scriberig_app::{
    ApplicationContext, ApplicationError, CapabilityDenial, DetailLevel, InspectRequest,
    InspectResult, QueryService, SnapshotResolution, SourceKind, ValidationState,
};
use scriberig_core::EntityId;

/// Selects the amount of trust metadata printed by inspection.
#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum InspectDepth {
    /// Print the compact provenance and validation summary.
    Compact,
    /// Include stable handles for deeper provenance inspection.
    Deep,
}

/// Executes an entity inspection through the shared application query boundary.
///
/// # Parameters
/// - `service`: Application query service selected by the product composition root.
/// - `context`: Caller and capability context for the request.
/// - `entity`: Entity identifier to inspect.
/// - `depth`: Compact or deep trust view.
///
/// # Returns
/// Rendered trust inspection output.
///
/// # Errors
/// Returns the structured application error returned by the query service.
pub fn inspect<S: QueryService>(
    service: &S,
    context: &ApplicationContext,
    entity: EntityId,
    depth: InspectDepth,
) -> Result<String, ApplicationError> {
    let request = InspectRequest {
        entity,
        snapshot: SnapshotResolution::Current,
        detail: match depth {
            InspectDepth::Compact => DetailLevel::Compact,
            InspectDepth::Deep => DetailLevel::Detailed,
        },
    };
    service
        .inspect(context, &request)
        .map(|result| render_inspection(&result, depth))
}

/// Renders an inspection result without exposing object content or secrets.
///
/// # Parameters
/// - `result`: Shared application inspection result.
/// - `depth`: Compact or deep trust view.
///
/// # Returns
/// Human-readable inspection output.
#[must_use]
pub fn render_inspection(result: &InspectResult, depth: InspectDepth) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "entity: {}", result.entity);
    if let Some(snapshot) = result.snapshot {
        let _ = writeln!(output, "snapshot: {snapshot}");
    }
    if let Some(revision) = result.revision {
        let _ = writeln!(output, "revision: {revision}");
    }
    if let Some(trust) = &result.trust {
        let _ = writeln!(output, "source: {}", source_label(&trust.source));
        let _ = writeln!(
            output,
            "validation: {}",
            validation_label(&trust.validation)
        );
        if let Some(accepted_via) = &trust.accepted_via {
            let _ = writeln!(output, "accepted via: {accepted_via}");
        }
        if depth == InspectDepth::Deep {
            for handle in &trust.provenance {
                let _ = writeln!(output, "provenance: {handle}");
            }
        }
    }
    output
}

/// Renders a structured capability denial without restricted object content.
///
/// # Parameters
/// - `denial`: Shared application denial details.
///
/// # Returns
/// Human-readable denial output.
#[must_use]
pub fn render_denial(denial: &CapabilityDenial) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "denied action: {}", denial.action);
    let _ = writeln!(output, "requested scope: {}", denial.scope);
    let _ = writeln!(output, "reason: {}", denial.reason);
    let _ = writeln!(output, "approval may grant: {}", denial.approval_may_grant);
    for alternative in &denial.alternatives {
        let _ = writeln!(output, "alternative: {alternative}");
    }
    output
}

/// Returns the stable display label for a source kind.
///
/// # Parameters
/// - `source`: Source kind to label.
///
/// # Returns
/// Stable machine-readable source label.
fn source_label(source: &SourceKind) -> &'static str {
    match source {
        SourceKind::Authored => "authored",
        SourceKind::Observed => "observed",
        SourceKind::Imported => "imported",
        SourceKind::Derived => "derived",
        SourceKind::ModelGenerated => "model_generated",
    }
}

/// Returns the stable display label for a validation state.
///
/// # Parameters
/// - `validation`: Validation state to label.
///
/// # Returns
/// Stable machine-readable validation label.
fn validation_label(validation: &ValidationState) -> &'static str {
    match validation {
        ValidationState::Current => "current",
        ValidationState::Stale => "stale",
        ValidationState::Failed => "failed",
        ValidationState::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::{InspectDepth, inspect, render_denial, render_inspection};
    use scriberig_app::{
        ApplicationContext, ApplicationError, CapabilityDenial, InspectRequest, InspectResult,
        QueryService, SourceKind, TrustSummary, ValidationState,
    };
    use scriberig_core::{EntityId, RevisionId, SnapshotId};

    /// Checks compact output contains provenance summary fields.
    #[test]
    fn compact_provenance_summary_is_rendered() {
        let output = render_inspection(&result(), InspectDepth::Compact);
        assert!(output.contains("source: model_generated"));
        assert!(output.contains("validation: current"));
        assert!(output.contains("accepted via: mig_1"));
        assert!(!output.contains("prov_1"));
    }

    /// Checks deep output follows provenance handles.
    #[test]
    fn deep_provenance_includes_handles() {
        let output = render_inspection(&result(), InspectDepth::Deep);
        assert!(output.contains("provenance: prov_1"));
    }

    /// Checks denials retain structured fields without secret values.
    #[test]
    fn denial_output_is_structured() {
        let denial = CapabilityDenial {
            action: "promote".to_owned(),
            scope: "entity:ent_1".to_owned(),
            reason: "capability missing".to_owned(),
            alternatives: vec!["inspect".to_owned()],
            approval_may_grant: true,
        };
        let output = render_denial(&denial);
        assert!(output.contains("denied action: promote"));
        assert!(output.contains("alternative: inspect"));
        assert!(!output.contains("secret"));
    }

    /// Checks CLI inspection delegates through the application query service.
    #[test]
    fn inspect_uses_shared_query_boundary() {
        let output = inspect(
            &FakeService,
            &ApplicationContext::empty(),
            EntityId::new([1; 16]),
            InspectDepth::Deep,
        )
        .expect("fake query service should succeed");
        assert!(output.contains("provenance: prov_1"));
    }

    /// Supplies deterministic inspection data for renderer tests.
    struct FakeService;

    impl QueryService for FakeService {
        /// Returns the deterministic inspection fixture.
        ///
        /// # Parameters
        /// - `self`: Deterministic test service.
        /// - `_context`: Unused application context.
        /// - `_request`: Unused inspection request.
        ///
        /// # Returns
        /// The inspection fixture.
        ///
        /// # Errors
        /// This fixture never returns an error.
        fn inspect(
            &self,
            _context: &ApplicationContext,
            _request: &InspectRequest,
        ) -> Result<InspectResult, ApplicationError> {
            Ok(result())
        }
    }

    /// Builds the inspection fixture used by renderer tests.
    ///
    /// # Returns
    /// A deterministic inspection result.
    ///
    /// # Errors
    /// This fixture construction cannot fail.
    fn result() -> InspectResult {
        InspectResult {
            entity: EntityId::new([1; 16]),
            revision: Some(RevisionId::new([2; 32])),
            snapshot: Some(SnapshotId::new([3; 32])),
            object_type: None,
            label: None,
            description: None,
            capabilities: Vec::new(),
            trust: Some(TrustSummary {
                source: SourceKind::ModelGenerated,
                validation: ValidationState::Current,
                accepted_via: Some("mig_1".to_owned()),
                provenance: vec!["prov_1".to_owned()],
                snapshot: Some(SnapshotId::new([3; 32])),
                revision: Some(RevisionId::new([2; 32])),
            }),
        }
    }
}
