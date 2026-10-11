//! Exercises the trust primitives that are currently implemented.

use scriberig_app::{
    AuthorizationDenial, Capability, CapabilityAction, CapabilityConstraints, CapabilityScope,
    ConflictSummary, ExternalEffectSummary, PolicyDecision, PolicyEvaluation, SourceKind,
    TrustSummary, ValidationState,
};
use scriberig_core::{EntityId, EntityRevisionRef, Provenance, ProvenanceSource, RevisionId};

/// Creates a deterministic trust fixture for application-level assertions.
///
/// # Returns
/// The trust summary, provenance record, and project inspection capability.
fn fixture() -> (TrustSummary, Provenance, Capability) {
    let entity = EntityId::from_bytes([1; 16]);
    let revision = RevisionId::new([2; 32]);
    let provenance = Provenance {
        source: ProvenanceSource::ModelGenerated,
        producer: Some("fixture-model".to_owned()),
        producer_version: None,
        inputs: vec![EntityRevisionRef {
            entity_id: entity,
            revision_id: revision,
        }],
        profile: Some("fixture-profile".to_owned()),
        note: Some("deterministic conformance fixture".to_owned()),
    };
    let trust = TrustSummary {
        source: SourceKind::ModelGenerated,
        validation: ValidationState::Current,
        accepted_via: Some("promotion-1".to_owned()),
        provenance: vec!["provenance-1".to_owned()],
        snapshot: None,
        revision: Some(revision),
        policy: Some(PolicyEvaluation {
            decision: PolicyDecision::Allowed,
            handle: Some("policy-1".to_owned()),
        }),
        approval: None,
        conflict: Some(ConflictSummary {
            handle: "conflict-1".to_owned(),
            resolvable: true,
        }),
        external_effect: Some(ExternalEffectSummary {
            handle: "receipt-1".to_owned(),
            state: "unknown".to_owned(),
        }),
    };
    let capability = Capability {
        scope: CapabilityScope::Project,
        actions: [CapabilityAction::Inspect].into_iter().collect(),
        constraints: CapabilityConstraints::unrestricted(),
    };
    (trust, provenance, capability)
}

/// Verifies generated knowledge retains exact provenance references.
#[test]
fn generated_knowledge_retains_provenance() {
    let (trust, provenance, _) = fixture();

    assert_eq!(trust.source, SourceKind::ModelGenerated);
    assert_eq!(provenance.inputs.len(), 1);
    assert_eq!(trust.revision, Some(RevisionId::new([2; 32])));
}

/// Verifies validation and external uncertainty remain explicit states.
#[test]
fn validation_and_external_uncertainty_remain_structured() {
    let (mut trust, _, _) = fixture();

    assert_eq!(trust.validation, ValidationState::Current);
    trust.validation = ValidationState::Stale;
    assert_eq!(trust.validation, ValidationState::Stale);
    assert_eq!(
        trust
            .external_effect
            .as_ref()
            .map(|effect| effect.state.as_str()),
        Some("unknown")
    );
}

/// Verifies delegation preserves least authority and structured denial data.
#[test]
fn capability_delegation_preserves_least_authority() {
    let (_, _, parent) = fixture();
    let entity = EntityId::from_bytes([1; 16]);
    let child = Capability {
        scope: CapabilityScope::Entity(entity),
        actions: [CapabilityAction::Inspect].into_iter().collect(),
        constraints: CapabilityConstraints::unrestricted(),
    };
    let broader = Capability {
        scope: CapabilityScope::Project,
        actions: [CapabilityAction::Promote].into_iter().collect(),
        constraints: CapabilityConstraints::unrestricted(),
    };

    assert!(parent.can_delegate(&child));
    assert!(!parent.can_delegate(&broader));
    let denial = AuthorizationDenial {
        principal: None,
        action: CapabilityAction::Promote,
        scope: CapabilityScope::Project,
        reason: "delegation would widen authority".to_owned(),
        alternatives: vec![CapabilityAction::Inspect],
        approval_possible: false,
    };
    assert!(!denial.approval_possible);
    assert_eq!(denial.alternatives, vec![CapabilityAction::Inspect]);
}
