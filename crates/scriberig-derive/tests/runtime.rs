//! Verifies production derived-runtime storage and resolution behavior.

use scriberig_core::{
    EntityId, EntityRevisionRef, KnowledgeValue, Provenance, ProvenanceSource, RevisionId,
};
use scriberig_derive::{
    Dependency, DependencySet, DerivationEvent, DerivationKey, DerivationProfile,
    DerivedRepository, DerivedResultHandle, DerivedResultRecord, DerivedResultStatus,
    DerivedRuntime, FreshnessPolicy, InMemoryDerivedRepository, SelectionPolicy,
};
use scriberig_graph::{DerivedArtifact, DerivedStatus};

/// Builds a deterministic test derivation key.
///
/// # Returns
/// A stable exact test key.
fn key() -> DerivationKey {
    DerivationKey::new(
        DerivationProfile::new("test:summary", "v1"),
        DependencySet::new(vec![Dependency::ExactRevision {
            entity: EntityId::from_bytes([1; 16]),
            revision: RevisionId::new([2; 32]),
        }]),
    )
}

/// Builds a successful test result record.
///
/// # Parameters
/// - `key`: Exact derivation key.
/// - `handle`: Single-byte value used to build a stable handle.
///
/// # Returns
/// A successful test result.
///
/// # Errors
/// This helper does not fail.
fn record(key: DerivationKey, handle: u8) -> DerivedResultRecord {
    DerivedResultRecord::new(
        key,
        DerivedResultHandle {
            artifact_id: RevisionId::new([handle; 32]),
        },
        DerivedResultStatus::Succeeded,
    )
}

/// Confirms records retain values and producer provenance.
#[test]
fn records_retain_payload_and_provenance() {
    let provenance = Provenance {
        source: ProvenanceSource::StaticAnalysis,
        producer: Some("test-analyzer".to_owned()),
        producer_version: Some("1".to_owned()),
        inputs: Vec::new(),
        profile: Some("test:summary/v1".to_owned()),
        note: Some("test result".to_owned()),
    };
    let result = record(key(), 3)
        .with_payload(KnowledgeValue::String("summary".to_owned()))
        .with_provenance(provenance.clone());

    assert_eq!(
        result.payload(),
        Some(&KnowledgeValue::String("summary".to_owned()))
    );
    assert_eq!(result.provenance(), &provenance);
}

/// Confirms universal graph artifacts can become cache records.
#[test]
fn records_wrap_universal_derived_artifacts() {
    let provenance = Provenance {
        source: ProvenanceSource::Compiler,
        producer: Some("test-compiler".to_owned()),
        producer_version: None,
        inputs: vec![EntityRevisionRef {
            entity_id: EntityId::from_bytes([1; 16]),
            revision_id: RevisionId::new([2; 32]),
        }],
        profile: None,
        note: None,
    };
    let artifact = DerivedArtifact {
        kind: "test:summary".to_owned(),
        dependencies: provenance.inputs.clone(),
        provenance: provenance.clone(),
        result: Some(KnowledgeValue::String("summary".to_owned())),
        status: DerivedStatus::Complete,
    };
    let result = DerivedResultRecord::from_artifact(
        key(),
        DerivedResultHandle {
            artifact_id: RevisionId::new([6; 32]),
        },
        artifact,
    );

    assert_eq!(result.status(), DerivedResultStatus::Succeeded);
    assert_eq!(
        result.payload(),
        Some(&KnowledgeValue::String("summary".to_owned()))
    );
    assert_eq!(result.provenance(), &provenance);
}

/// Confirms an eligible exact result is reused without recomputation.
#[test]
fn runtime_reuses_exact_result_without_recomputing() {
    let key = key();
    let existing = record(key.clone(), 4);
    let mut repository = InMemoryDerivedRepository::new();
    repository.insert(existing.clone()).unwrap();
    let mut runtime = DerivedRuntime::new(repository);
    let policy = SelectionPolicy {
        profile: Some(key.profile.clone()),
        include_deterministic_failures: false,
        require_fresh: false,
        now: 0,
    };
    let outcome = runtime
        .resolve(&key, &policy, &FreshnessPolicy::Immutable, |_key| {
            panic!("exact reusable result must not be recomputed")
        })
        .unwrap();

    assert!(outcome.reused());
    assert_eq!(outcome.record(), &existing);
}

/// Confirms a cache miss computes and stores the produced result.
#[test]
fn runtime_computes_and_stores_a_missed_result() {
    let key = key();
    let mut runtime = DerivedRuntime::new(InMemoryDerivedRepository::new());
    let policy = SelectionPolicy {
        profile: Some(key.profile.clone()),
        include_deterministic_failures: false,
        require_fresh: false,
        now: 0,
    };
    let computed = record(key.clone(), 5);
    let outcome = runtime
        .resolve(&key, &policy, &FreshnessPolicy::Immutable, |_key| {
            computed.clone()
        })
        .unwrap();

    assert!(!outcome.reused());
    assert_eq!(outcome.record(), &computed);
    assert_eq!(runtime.repository().get(computed.handle()), Some(&computed));
}

/// Confirms runtime resolution exposes derived lifecycle events separately.
#[test]
fn runtime_emits_selection_and_availability_events() {
    let key = key();
    let existing = record(key.clone(), 8);
    let mut runtime = DerivedRuntime::new(InMemoryDerivedRepository::new());
    let policy = SelectionPolicy {
        profile: Some(key.profile.clone()),
        include_deterministic_failures: false,
        require_fresh: false,
        now: 0,
    };

    let (resolution, events) = runtime
        .resolve_with_events(&key, &policy, &FreshnessPolicy::Immutable, |_key| {
            existing.clone()
        })
        .unwrap();

    assert!(!resolution.reused());
    assert!(matches!(
        events.as_slice(),
        [
            DerivationEvent::DerivationStarted { key: started, .. },
            DerivationEvent::DerivedResultAvailable { result, .. }
        ] if *started == key && *result == existing.handle()
    ));
}

/// Confirms reuse emits a selection event without recomputing or mutating state.
#[test]
fn runtime_emits_selection_event_for_reuse() {
    let key = key();
    let existing = record(key.clone(), 9);
    let mut repository = InMemoryDerivedRepository::new();
    repository.insert(existing.clone()).unwrap();
    let mut runtime = DerivedRuntime::new(repository);
    let policy = SelectionPolicy {
        profile: Some(key.profile.clone()),
        include_deterministic_failures: false,
        require_fresh: false,
        now: 0,
    };

    let (resolution, events) = runtime
        .resolve_with_events(&key, &policy, &FreshnessPolicy::Immutable, |_key| {
            panic!("reusable result must not be recomputed")
        })
        .unwrap();

    assert!(resolution.reused());
    assert!(matches!(
        events.as_slice(),
        [
            DerivationEvent::DerivationStarted { .. },
            DerivationEvent::DerivedResultSelected { result, .. }
        ] if *result == existing.handle()
    ));
}
