//! Verifies explicit integration of durable project dependency candidates.

use super::*;

/// Integrates disjoint candidates while retaining a reviewable exact-base result.
#[test]
fn candidate_merge_combines_disjoint_dependencies_without_promotion() {
    let (root, store, project, snapshot, _) = super::setup("candidate-merge");
    let service = ProjectService::new(Arc::clone(&store));
    let baseline = fs::read(root.join("src/lib.rs")).expect("baseline source");
    let left = service
        .propose_project_change(
            project,
            snapshot,
            vec![ProjectSourceChange {
                path: "src/lib.rs".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"pub fn left() {}\n".to_vec()),
            }],
        )
        .expect("left candidate");
    let right = service
        .propose_project_change(
            project,
            snapshot,
            vec![ProjectSourceChange {
                path: "Cargo.toml".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(
                    b"[package]\nname=\"golden\"\nversion=\"0.2.0\"\nedition=\"2021\"\n".to_vec(),
                ),
            }],
        )
        .expect("right candidate");

    let integrated = service
        .merge_project_candidates(&left.id, &right.id)
        .expect("integrate dependency candidates");

    assert_eq!(integrated.base_snapshot, snapshot);
    assert_eq!(integrated.effects.len(), 2);
    assert_eq!(
        integrated.validation,
        scriberig_project::CandidateValidation::Pending
    );
    assert_eq!(
        service.workspace(project).expect("workspace").snapshot,
        snapshot
    );
    assert_eq!(
        fs::read(root.join("src/lib.rs")).expect("unchanged source"),
        baseline
    );
}

/// Rejects same-path dependency edits as a structured integration conflict.
#[test]
fn candidate_merge_reports_same_target_conflict() {
    let (_root, store, project, snapshot, _) = super::setup("candidate-merge-conflict");
    let service = ProjectService::new(store);
    let left = service
        .propose_project_change(
            project,
            snapshot,
            vec![ProjectSourceChange {
                path: "src/lib.rs".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"pub fn left() {}\n".to_vec()),
            }],
        )
        .expect("left candidate");
    let right = service
        .propose_project_change(
            project,
            snapshot,
            vec![ProjectSourceChange {
                path: "src/lib.rs".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"pub fn right() {}\n".to_vec()),
            }],
        )
        .expect("right candidate");

    let error = service
        .merge_project_candidates(&left.id, &right.id)
        .expect_err("same-target conflict");

    assert_eq!(error.code, "candidate_merge_conflict");
    assert!(error.message.contains("src/lib.rs"));
}
