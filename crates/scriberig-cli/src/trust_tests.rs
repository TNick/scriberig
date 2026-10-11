//! Tests for trust inspection rendering and dispatch.

use super::{
    AUTHORIZATION_DENIED_EXIT_CODE, InspectDepth, TrustCommandError, inspect, inspect_at,
    render_denial, render_denial_json, render_inspection, render_inspection_json,
};
use scriberig_app::{
    ApplicationContext, ApplicationError, CapabilityDenial, InspectRequest, InspectResult,
    QueryService, SnapshotResolution, SourceKind, TrustSummary, ValidationState,
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

/// Checks stale and failed validation states remain distinct in output.
#[test]
fn validation_states_remain_distinct() {
    let mut stale = result();
    stale.trust.as_mut().expect("trust fixture").validation = ValidationState::Stale;
    assert!(render_inspection(&stale, InspectDepth::Compact).contains("validation: stale"));

    let mut failed = result();
    failed.trust.as_mut().expect("trust fixture").validation = ValidationState::Failed;
    assert!(render_inspection(&failed, InspectDepth::Compact).contains("validation: failed"));
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

/// Checks JSON output preserves structured trust fields and omits hidden data.
#[test]
fn machine_readable_output_is_structured() {
    let output = render_inspection_json(&result(), InspectDepth::Deep);
    assert!(output.starts_with('{'));
    assert!(output.contains("\"source\":\"model_generated\""));
    assert!(output.contains("\"provenance\":[\"prov_1\"]"));
    assert!(!output.contains("secret"));

    let denial = CapabilityDenial {
        action: "promote".to_owned(),
        scope: "entity:ent_1".to_owned(),
        reason: "capability missing".to_owned(),
        alternatives: vec!["inspect".to_owned()],
        approval_may_grant: true,
    };
    let denial_output = render_denial_json(&denial);
    assert!(denial_output.contains("\"approval_may_grant\":true"));
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

/// Checks authorization denial gets a distinct process status.
#[test]
fn denied_inspection_has_authorization_exit_status() {
    let error = inspect_at(
        &DenyingService,
        &ApplicationContext::empty(),
        EntityId::new([1; 16]),
        SnapshotResolution::Current,
        InspectDepth::Compact,
    )
    .expect_err("denying service should reject inspection");
    assert_eq!(error.exit_code(), AUTHORIZATION_DENIED_EXIT_CODE);
    assert!(matches!(error, TrustCommandError::AuthorizationDenied(_)));
}

/// Supplies deterministic inspection data for renderer tests.
struct FakeService;

/// Rejects every inspection request for authorization tests.
struct DenyingService;

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

impl QueryService for DenyingService {
    /// Returns a structured permission failure.
    ///
    /// # Parameters
    /// - `self`: Denying test service.
    /// - `_context`: Unused application context.
    /// - `_request`: Unused inspection request.
    ///
    /// # Returns
    /// Never returns a successful inspection.
    ///
    /// # Errors
    /// Always returns a permission denial.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        _request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        Err(ApplicationError::PermissionDenied {
            action: "inspect".to_owned(),
            scope: "entity".to_owned(),
        })
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
            policy: None,
            approval: None,
            conflict: None,
            external_effect: None,
        }),
    }
}
