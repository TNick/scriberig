//! Verifies capabilities in the projection contract.

use sr_projection::capabilities::CapabilityError;
use sr_projection::{
    ProjectionCapabilities, ProjectionFeature, ProjectionKind, ReconciliationOutcome,
};

/// Checks that editable capabilities cannot omit parsing support.
#[test]
fn editable_capabilities_require_parsing() {
    let result =
        ProjectionCapabilities::new([ProjectionFeature::Renderable, ProjectionFeature::Editable]);
    assert_eq!(result, Err(CapabilityError::EditableRequiresParseable));
}

/// Checks that read-only output does not require parsing support.
#[test]
fn read_only_projection_can_render_without_parsing() {
    let capabilities = ProjectionCapabilities::read_only();
    assert!(capabilities.supports(ProjectionFeature::Renderable));
    assert!(!capabilities.supports(ProjectionFeature::Parseable));
}

/// Checks that projection kinds accept adapter-defined identifiers.
#[test]
fn projection_kind_is_extensible() {
    let kind = ProjectionKind::new("vendor.custom-view.v1").unwrap();
    assert_eq!(kind.as_str(), "vendor.custom-view.v1");
}

/// Checks reconciliation states can be distinguished by enum variant.
#[test]
fn reconciliation_outcomes_are_typed() {
    let outcome: ReconciliationOutcome<(), (), ()> = ReconciliationOutcome::NoSemanticChange;
    assert!(matches!(outcome, ReconciliationOutcome::NoSemanticChange));
    let proposal: ReconciliationOutcome<(), (), ()> = ReconciliationOutcome::MigrationProposal(());
    assert!(matches!(
        proposal,
        ReconciliationOutcome::MigrationProposal(())
    ));
    let ambiguous: ReconciliationOutcome<(), (), &str> = ReconciliationOutcome::Ambiguous("detail");
    assert!(matches!(ambiguous, ReconciliationOutcome::Ambiguous(_)));
    let conflict: ReconciliationOutcome<(), (), ()> = ReconciliationOutcome::Conflict(());
    assert!(matches!(conflict, ReconciliationOutcome::Conflict(())));
    let invalid: ReconciliationOutcome<(), (), &str> =
        ReconciliationOutcome::InvalidExternalState("detail");
    assert!(matches!(
        invalid,
        ReconciliationOutcome::InvalidExternalState(_)
    ));
}

/// Checks generic projection kinds and classes contain no domain syntax.
#[test]
fn generic_model_is_domain_neutral() {
    let kind = ProjectionKind::new("application/x-example").unwrap();
    assert_eq!(kind.as_str(), "application/x-example");
}
