//! Verifies freshness, selection, failure, and retention independence.

use super::support::{TestRepository, entity, handle, key, revision, successful};
use scriberig_core::SnapshotId;
use scriberig_derive::{
    Dependency, DerivedRepository, DerivedResultRecord, DerivedResultStatus, FailureReason,
    FreshnessObservation, FreshnessPolicy, FreshnessState, RetentionPolicy, RetentionStrength,
    SelectionOutcome, SelectionPolicy, SelectionReason, evaluate_freshness, select,
};

/// Proves external observations age without losing exact historical retrieval.
#[test]
fn external_observation_expires_for_current_requests_only() {
    let freshness = FreshnessPolicy::SourcePolicy("source-x/v1".into());
    let observation = FreshnessObservation {
        source: "source-x".into(),
        observed_at: 100,
        valid_until: Some(200),
        validator: Some("etag-1".into()),
    };
    let result = successful(
        key(
            "external-observation",
            vec![Dependency::Scope("source-x".into())],
        ),
        41,
    )
    .with_freshness(observation.clone());
    let mut repository = TestRepository::default();
    repository.insert(result.clone()).unwrap();

    assert_eq!(
        evaluate_freshness(&freshness, Some(&observation), 199),
        FreshnessState::Fresh
    );
    assert_eq!(
        evaluate_freshness(&freshness, Some(&observation), 200),
        FreshnessState::Stale
    );
    let policy = SelectionPolicy {
        profile: Some(result.key().profile.clone()),
        include_deterministic_failures: false,
        require_fresh: true,
        now: 200,
    };
    assert_eq!(
        select(std::slice::from_ref(&result), &policy, &freshness),
        SelectionOutcome::NoMatch {
            reason: SelectionReason::FreshnessExpired
        }
    );
    assert_eq!(repository.get(handle(41)), Some(&result));
}

/// Proves multiple immutable outcomes coexist and selection order is explicit.
#[test]
fn multiple_results_coexist_and_selection_does_not_mutate_them() {
    let request = key(
        "nondeterministic-summary",
        vec![Dependency::ExactRevision {
            entity: entity(7),
            revision: revision(7),
        }],
    );
    let first = successful(request.clone(), 51);
    let second = successful(request.clone(), 52);
    let mut repository = TestRepository::default();
    repository.insert(first.clone()).unwrap();
    repository.insert(second.clone()).unwrap();
    let snapshot_before = SnapshotId::new([8; 32]);
    let policy = SelectionPolicy {
        profile: Some(request.profile.clone()),
        include_deterministic_failures: false,
        require_fresh: true,
        now: 1,
    };
    let candidates_a = [first.clone(), second.clone()];
    let candidates_b = [second.clone(), first.clone()];
    let selection_a = select(&candidates_a, &policy, &FreshnessPolicy::Immutable);
    let selection_b = select(&candidates_b, &policy, &FreshnessPolicy::Immutable);

    assert_eq!(repository.by_key(&request).len(), 2);
    assert!(
        matches!(selection_a, SelectionOutcome::Selected { record } if record.handle() == first.handle())
    );
    assert!(
        matches!(selection_b, SelectionOutcome::Selected { record } if record.handle() == second.handle())
    );
    assert_eq!(repository.get(first.handle()), Some(&first));
    assert_eq!(repository.get(second.handle()), Some(&second));
    assert_eq!(snapshot_before, SnapshotId::new([8; 32]));
}

/// Proves retry/reuse policy follows structured status and failure categories.
#[test]
fn failure_policy_uses_status_not_message_text() {
    let request = key(
        "fake-parser",
        vec![Dependency::ExactRevision {
            entity: entity(8),
            revision: revision(8),
        }],
    );
    let deterministic = DerivedResultRecord::new(
        request.clone(),
        handle(61),
        DerivedResultStatus::FailedDeterministic,
    )
    .with_failure(FailureReason::InvalidInput);
    let transient = DerivedResultRecord::new(
        request.clone(),
        handle(62),
        DerivedResultStatus::FailedTransient,
    )
    .with_failure(FailureReason::TemporaryUnavailable);
    let inspect = SelectionPolicy {
        profile: Some(request.profile.clone()),
        include_deterministic_failures: true,
        require_fresh: false,
        now: 0,
    };
    let successful_only = SelectionPolicy {
        include_deterministic_failures: false,
        ..inspect.clone()
    };

    assert_eq!(
        deterministic.status(),
        DerivedResultStatus::FailedDeterministic
    );
    assert_eq!(deterministic.failure(), Some(&FailureReason::InvalidInput));
    assert!(matches!(
        select(
            std::slice::from_ref(&deterministic),
            &inspect,
            &FreshnessPolicy::Immutable
        ),
        SelectionOutcome::Selected { .. }
    ));
    assert_eq!(
        select(&[transient], &inspect, &FreshnessPolicy::Immutable),
        SelectionOutcome::NoMatch {
            reason: SelectionReason::StatusNotReusable
        }
    );
    assert_eq!(
        select(
            &[deterministic],
            &successful_only,
            &FreshnessPolicy::Immutable
        ),
        SelectionOutcome::NoMatch {
            reason: SelectionReason::StatusNotReusable
        }
    );
}

/// Proves retention/reference strength are independent from exact validity.
#[test]
fn retention_strength_is_independent_from_validity() {
    let request = key(
        "summary",
        vec![Dependency::ExactRevision {
            entity: entity(9),
            revision: revision(9),
        }],
    );
    let disposable = successful(request.clone(), 71);
    let pinned = successful(request, 72).with_retention(
        RetentionPolicy::Pinned,
        RetentionStrength::StrongPinnedDependency,
    );
    let mut repository = TestRepository::default();
    repository.insert(disposable.clone()).unwrap();
    repository.insert(pinned.clone()).unwrap();

    assert_eq!(
        disposable.retention(),
        (
            RetentionPolicy::Disposable,
            RetentionStrength::WeakDependency
        )
    );
    assert_eq!(
        pinned.retention(),
        (
            RetentionPolicy::Pinned,
            RetentionStrength::StrongPinnedDependency
        )
    );
    assert_eq!(disposable.key(), pinned.key());
    assert!(repository.remove_disposable(disposable.handle()));
    assert!(!repository.remove_disposable(pinned.handle()));
    assert_eq!(repository.get(pinned.handle()), Some(&pinned));
}
