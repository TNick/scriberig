//! Verifies event separation and extensible generic-domain registration.

use super::support::{entity, handle, key, revision};
use scriberig_core::SnapshotId;
use scriberig_derive::{
    Applicability, Dependency, DependencyDiscovery, DerivationEvent, DerivationProfile,
    DerivationRegistration, DerivationRegistry, DerivationSchedule, FreshnessPolicy,
    ProducerIdentity, RegistrationError, RetentionPolicy,
};

/// Proves derived lifecycle values carry identifiers without snapshot mutation.
#[test]
fn lifecycle_events_are_derived_only_and_do_not_advance_snapshot() {
    let request = key(
        "summary",
        vec![Dependency::ExactRevision {
            entity: entity(10),
            revision: revision(10),
        }],
    );
    let result = handle(81);
    let snapshot_before = SnapshotId::new([12; 32]);
    let events = [
        DerivationEvent::DerivationStarted {
            profile: request.profile.clone(),
            key: request.clone(),
            subject: Some(entity(10)),
        },
        DerivationEvent::DerivedResultAvailable {
            profile: request.profile.clone(),
            key: request.clone(),
            result,
            subject: Some(entity(10)),
        },
        DerivationEvent::DerivedResultSelected {
            profile: request.profile.clone(),
            key: request.clone(),
            result,
            subject: Some(entity(10)),
        },
    ];

    assert!(
        matches!(&events[0], DerivationEvent::DerivationStarted { key, .. } if key == &request)
    );
    assert!(
        matches!(&events[1], DerivationEvent::DerivedResultAvailable { result: found, .. } if *found == result)
    );
    assert!(
        matches!(&events[2], DerivationEvent::DerivedResultSelected { key, result: found, .. } if key == &request && *found == result)
    );
    assert_eq!(snapshot_before, SnapshotId::new([12; 32]));
}

/// Proves fake universal type IDs register without production domain enums.
#[test]
fn fake_domain_registration_remains_extensible() {
    let profile =
        DerivationProfile::new("test:custom-summary", "v1").with_schedule(DerivationSchedule::Lazy);
    let registration = DerivationRegistration::new(
        profile.clone(),
        vec![Applicability::EntityType("urn:test:entity:custom".into())],
        DependencyDiscovery::ProducerSupplied,
        ProducerIdentity::new("fake-domain/impl-1"),
    )
    .with_retention(RetentionPolicy::HistoricalEvidence);
    let mut registry = DerivationRegistry::new();
    registry.register(registration.clone()).unwrap();

    assert_eq!(registry.lookup(&profile), Some(&registration));
    assert_eq!(
        registry
            .applicable_to(&Applicability::EntityType("urn:test:entity:custom".into()))
            .len(),
        1
    );
    assert_eq!(registration.schedule(), DerivationSchedule::Lazy);
    assert_eq!(
        registration.dependency_discovery(),
        &DependencyDiscovery::ProducerSupplied
    );
    assert_eq!(registration.freshness(), &FreshnessPolicy::Immutable);
    assert_eq!(
        registration.retention(),
        RetentionPolicy::HistoricalEvidence
    );
    assert_eq!(registration.producer().as_str(), "fake-domain/impl-1");
    let conflicting = registration.clone().with_retention(RetentionPolicy::Pinned);
    assert_eq!(
        registry.register(conflicting),
        Err(RegistrationError::ConflictingDuplicate { profile })
    );
}
