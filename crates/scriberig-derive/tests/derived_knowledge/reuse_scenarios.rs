//! Verifies local, global, scoped, styled, and hierarchical dependency reuse.

use super::support::{TestRepository, entity, handle, key, revision, successful};
use scriberig_core::SnapshotId;
use scriberig_derive::{
    Dependency, DerivedRepository, ExplanationReason, SemanticChange, affected_transitively,
    explain_key_comparison, is_affected,
};

/// Proves exact dependency identity is stable and revision-sensitive.
#[test]
fn exact_revision_identity_changes_only_when_revision_changes() {
    let first = key(
        "summary",
        vec![Dependency::ExactRevision {
            entity: entity(11),
            revision: revision(1),
        }],
    );
    let same = key(
        "summary",
        vec![Dependency::ExactRevision {
            entity: entity(11),
            revision: revision(1),
        }],
    );
    let changed = key(
        "summary",
        vec![Dependency::ExactRevision {
            entity: entity(11),
            revision: revision(2),
        }],
    );

    assert_eq!(first, same);
    assert_ne!(first, changed);
    assert!(matches!(
        explain_key_comparison(&first, &changed).reason,
        ExplanationReason::RevisionChanged { entity: changed } if changed == entity(11)
    ));
}

/// Proves local exact dependencies remain reusable after unrelated changes.
#[test]
fn local_result_reuses_after_unrelated_global_change() {
    let initial_snapshot = SnapshotId::new([10; 32]);
    let later_snapshot = SnapshotId::new([11; 32]);
    let local_key = key(
        "summary",
        vec![Dependency::ExactRevision {
            entity: entity(1),
            revision: revision(1),
        }],
    );
    let unrelated = SemanticChange::Revision {
        entity: entity(2),
        from: revision(2),
        to: revision(3),
    };
    let result = successful(local_key.clone(), 21);
    let mut repository = TestRepository::default();
    repository.insert(result.clone()).unwrap();

    assert_ne!(initial_snapshot, later_snapshot);
    assert!(!is_affected(&result, &unrelated));
    assert_eq!(
        explain_key_comparison(&local_key, &local_key).reason,
        ExplanationReason::ExactKeyMatch
    );
    assert_eq!(repository.by_key(&local_key), vec![&result]);
    assert_eq!(repository.get(handle(21)), Some(&result));
}

/// Proves explicit whole-snapshot dependencies change keys but preserve history.
#[test]
fn explicit_snapshot_dependency_changes_key_and_preserves_history() {
    let before = SnapshotId::new([1; 32]);
    let after = SnapshotId::new([2; 32]);
    let old_key = key("project-audit", vec![Dependency::Snapshot(before)]);
    let new_key = key("project-audit", vec![Dependency::Snapshot(after)]);
    let old_result = successful(old_key.clone(), 22);
    let mut repository = TestRepository::default();
    repository.insert(old_result.clone()).unwrap();
    let explanation = explain_key_comparison(&old_key, &new_key);

    assert_ne!(old_key, new_key);
    assert_eq!(repository.get(handle(22)), Some(&old_result));
    assert_eq!(
        repository.by_key(&new_key),
        Vec::<&scriberig_derive::DerivedResultRecord>::new()
    );
    assert!(
        matches!(explanation.reason, ExplanationReason::SnapshotChanged { from, to } if from == before && to == after)
    );
}

/// Proves one-sided snapshot dependencies retain truthful diagnostic values.
#[test]
fn snapshot_dependency_addition_and_removal_are_explained() {
    let snapshot = SnapshotId::new([3; 32]);
    let empty = key("audit", Vec::new());
    let scoped = key("audit", vec![Dependency::Snapshot(snapshot)]);

    assert!(matches!(
        explain_key_comparison(&scoped, &empty).reason,
        ExplanationReason::SnapshotDependencyRemoved { from } if from == snapshot
    ));
    assert!(matches!(
        explain_key_comparison(&empty, &scoped).reason,
        ExplanationReason::SnapshotDependencyAdded { to } if to == snapshot
    ));
}

/// Proves a scope dependency detects new members when old members stay unchanged.
#[test]
fn scope_dependency_catches_new_matching_member() {
    let a = entity(1);
    let b = entity(2);
    let c = entity(3);
    let old_scope = "matching-members@r1";
    let new_scope = "matching-members@r2";
    let old_key = key(
        "all-matches",
        vec![
            Dependency::ExactRevision {
                entity: a,
                revision: revision(1),
            },
            Dependency::ExactRevision {
                entity: b,
                revision: revision(2),
            },
            Dependency::Scope(old_scope.into()),
        ],
    );
    let result = successful(old_key.clone(), 23);
    let change = SemanticChange::Scope {
        from: old_scope.into(),
        to: new_scope.into(),
    };
    let new_key = key(
        "all-matches",
        vec![
            Dependency::ExactRevision {
                entity: a,
                revision: revision(1),
            },
            Dependency::ExactRevision {
                entity: b,
                revision: revision(2),
            },
            Dependency::Scope(new_scope.into()),
        ],
    );

    assert_ne!(a, c);
    assert_ne!(b, c);
    assert!(is_affected(&result, &change));
    assert_eq!(
        explain_key_comparison(&old_key, &new_key).reason,
        ExplanationReason::ScopeChanged {
            scope: old_scope.into()
        }
    );
}

/// Proves a style-only change invalidates rendering, not semantic summaries.
#[test]
fn style_only_change_is_local_to_render_derivation() {
    let subject = Dependency::ExactRevision {
        entity: entity(4),
        revision: revision(4),
    };
    let summary = successful(key("summary", vec![subject.clone()]), 24);
    let render_old = key(
        "render",
        vec![
            subject.clone(),
            Dependency::Configuration("style:ST1".into()),
        ],
    );
    let render_new = key(
        "render",
        vec![subject, Dependency::Configuration("style:ST2".into())],
    );
    let rendered = successful(render_old.clone(), 25);
    let change = SemanticChange::Configuration {
        from: "style:ST1".into(),
        to: "style:ST2".into(),
    };

    assert!(!is_affected(&summary, &change));
    assert!(is_affected(&rendered, &change));
    assert_eq!(
        explain_key_comparison(&render_old, &render_new).reason,
        ExplanationReason::ConfigurationChanged {
            from: "style:ST1".into(),
            to: "style:ST2".into()
        }
    );
    assert_eq!(summary.key().dependencies.as_slice().len(), 1);
}

/// Proves transitive invalidation preserves reusable sibling results and history.
#[test]
fn hierarchical_reuse_keeps_unaffected_child_and_old_results() {
    let first_input = Dependency::ExactRevision {
        entity: entity(5),
        revision: revision(5),
    };
    let second_old_input = Dependency::ExactRevision {
        entity: entity(6),
        revision: revision(6),
    };
    let second_new_input = Dependency::ExactRevision {
        entity: entity(6),
        revision: revision(7),
    };
    let first_summary = successful(key("summary", vec![first_input]), 31);
    let second_old = successful(key("summary", vec![second_old_input]), 32);
    let module_old = successful(
        key(
            "module-summary",
            vec![
                Dependency::DerivedResult(first_summary.handle().artifact_id),
                Dependency::DerivedResult(second_old.handle().artifact_id),
            ],
        ),
        33,
    );
    let second_new = successful(key("summary", vec![second_new_input]), 34);
    let module_new = successful(
        key(
            "module-summary",
            vec![
                Dependency::DerivedResult(first_summary.handle().artifact_id),
                Dependency::DerivedResult(second_new.handle().artifact_id),
            ],
        ),
        35,
    );
    let change = SemanticChange::Revision {
        entity: entity(6),
        from: revision(6),
        to: revision(7),
    };
    let mut repository = TestRepository::default();
    repository.insert(first_summary.clone()).unwrap();
    repository.insert(second_old.clone()).unwrap();
    repository.insert(module_old.clone()).unwrap();
    repository.insert(second_new.clone()).unwrap();
    repository.insert(module_new.clone()).unwrap();
    let history = [
        first_summary.clone(),
        second_old.clone(),
        module_old.clone(),
    ];
    let affected = affected_transitively(&history, &change);

    assert!(!is_affected(&first_summary, &change));
    assert!(affected.contains(&second_old.handle()));
    assert!(affected.contains(&module_old.handle()));
    assert_eq!(repository.get(first_summary.handle()), Some(&first_summary));
    assert_eq!(repository.get(second_old.handle()), Some(&second_old));
    assert_eq!(repository.get(module_old.handle()), Some(&module_old));
    assert_eq!(
        second_old.key().dependencies.as_slice(),
        &[Dependency::ExactRevision {
            entity: entity(6),
            revision: revision(6)
        }]
    );
    assert_eq!(module_old.key().dependencies.as_slice().len(), 2);
    assert_eq!(module_new.key().dependencies.as_slice().len(), 2);
    assert!(
        module_new
            .key()
            .dependencies
            .as_slice()
            .contains(&Dependency::DerivedResult(
                first_summary.handle().artifact_id
            ))
    );
    assert!(
        module_new
            .key()
            .dependencies
            .as_slice()
            .contains(&Dependency::DerivedResult(second_new.handle().artifact_id))
    );
    assert_ne!(module_old.handle(), module_new.handle());
}
