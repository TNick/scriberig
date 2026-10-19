//! Tests registration compatibility, deterministic duplicates, and defaults.

use super::{
    Applicability, CostHint, DependencyDiscovery, DerivationRegistration, DerivationRegistry,
    ProducerIdentity, RegistrationError,
};
use crate::freshness::FreshnessPolicy;
use crate::profile::{DerivationProfile, DerivationSchedule};
use crate::retention::RetentionPolicy;

/// Builds a generic custom-domain registration for tests.
///
/// - `version`: Exact fake profile version.
///
/// Test registration metadata.
/// # Returns
/// The operation result.
fn registration(version: &str) -> DerivationRegistration {
    DerivationRegistration::new(
        DerivationProfile::new("test-domain/summary", version),
        vec![Applicability::EntityType("urn:test:entity:note".into())],
        DependencyDiscovery::DeclaredExactInputs,
        ProducerIdentity::new("test-implementation/1"),
    )
    .with_schedule(DerivationSchedule::Eager)
    .with_freshness(FreshnessPolicy::SourcePolicy("test-source-v1".into()))
    .with_retention(RetentionPolicy::ReusableExpensive)
    .with_cost(CostHint { units: 12 })
}

/// Accepts custom profiles and universal type identifiers.
#[test]
fn accepts_custom_domain_profile() {
    let mut registry = DerivationRegistry::new();
    assert_eq!(registry.register(registration("v1")), Ok(()));
    assert_eq!(registry.len(), 1);
}

/// Looks up registrations by complete exact profile identity.
#[test]
fn exact_profile_lookup_works() {
    let mut registry = DerivationRegistry::new();
    let definition = registration("v1");
    let profile = definition.profile().clone();
    registry.register(definition).unwrap();

    assert_eq!(registry.lookup(&profile).unwrap().profile(), &profile);
    assert!(registry.lookup(registration("v2").profile()).is_none());
}

/// Lists profiles by generic entity type and scope IDs.
#[test]
fn applicable_listing_uses_universal_type_ids() {
    let mut registry = DerivationRegistry::new();
    registry.register(registration("v1")).unwrap();
    registry.register(registration("v2")).unwrap();
    let matches = registry.applicable_to(&Applicability::EntityType("urn:test:entity:note".into()));

    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].profile().version(), "v1");
    assert_eq!(
        registry.applicable_to(&Applicability::ScopeKind("urn:test:scope".into())),
        Vec::<&DerivationRegistration>::new()
    );
}

/// Rejects conflicting duplicates without replacing the original metadata.
#[test]
fn conflicting_duplicate_registration_fails_structurally() {
    let mut registry = DerivationRegistry::new();
    let definition = registration("v1");
    let profile = definition.profile().clone();
    registry.register(definition.clone()).unwrap();
    let conflicting = definition.clone().with_retention(RetentionPolicy::Pinned);

    assert_eq!(
        registry.register(conflicting),
        Err(RegistrationError::ConflictingDuplicate {
            profile: profile.clone()
        })
    );
    assert_eq!(registry.lookup(&profile), Some(&definition));
}

/// Makes identical duplicates and insertion order deterministic.
#[test]
fn identical_duplicates_are_deterministic() {
    let first = registration("v1");
    let second = registration("v2");
    let mut left = DerivationRegistry::new();
    let mut right = DerivationRegistry::new();
    left.register(first.clone()).unwrap();
    left.register(first.clone()).unwrap();
    left.register(second.clone()).unwrap();
    right.register(second).unwrap();
    right.register(first).unwrap();

    assert_eq!(left, right);
    assert_eq!(left.len(), 2);
}

/// Exposes schedule, policies, discovery strategy, producer, and cost.
#[test]
fn registration_metadata_is_inspectable() {
    let definition = registration("v1");

    assert_eq!(definition.schedule(), DerivationSchedule::Eager);
    assert_eq!(
        definition.freshness(),
        &FreshnessPolicy::SourcePolicy("test-source-v1".into())
    );
    assert_eq!(definition.retention(), RetentionPolicy::ReusableExpensive);
    assert_eq!(
        definition.dependency_discovery(),
        &DependencyDiscovery::DeclaredExactInputs
    );
    assert_eq!(definition.producer().as_str(), "test-implementation/1");
    assert_eq!(definition.cost(), Some(CostHint { units: 12 }));
}

/// Uses generic defaults and inherits a lazy profile schedule.
#[test]
fn defaults_are_generic_and_inherit_profile_schedule() {
    let definition = DerivationRegistration::new(
        DerivationProfile::new("any-domain", "revision-3"),
        vec![Applicability::ScopeKind("urn:test:scope".into())],
        DependencyDiscovery::WholeSnapshot,
        ProducerIdentity::new("impl/v3"),
    );

    assert_eq!(definition.schedule(), DerivationSchedule::Lazy);
    assert_eq!(definition.freshness(), &FreshnessPolicy::Immutable);
    assert_eq!(definition.retention(), RetentionPolicy::Disposable);
    assert_eq!(
        definition.dependency_discovery(),
        &DependencyDiscovery::WholeSnapshot
    );
}
