//! Verifies durable PHP candidate application and semantic restart evidence.

use std::fs;
use std::sync::Arc;

use scriberig_project::ProjectService;
use scriberig_store::SnapshotStore;
use scriberig_store_sqlite::SqliteStore;

/// Verifies durable candidate validation, application receipts, and snapshot lineage.
///
/// # Parameters
/// - `database`: Reopened product database.
/// - `project`: Exact project identity.
/// - `base`: Candidate base snapshot.
/// - `candidate_id`: Candidate identity returned by the root agent result.
/// - `super`: Parent PTY test module.
///
/// # Returns
/// Applied successor snapshot recorded by the durable effect receipt.
pub(super) fn assert_durable_candidate_effect(
    database: &std::path::Path,
    project: scriberig_core::ProjectId,
    base: scriberig_core::SnapshotId,
    candidate_id: &str,
) -> scriberig_core::SnapshotId {
    let store = SqliteStore::open(database).expect("open durable workflow store");
    let candidate = ProjectService::new(Arc::new(
        SqliteStore::open(database).expect("project store"),
    ))
    .candidate(candidate_id)
    .expect("read durable candidate")
    .expect("candidate survives restart");
    assert_eq!(candidate.project, project);
    assert_eq!(candidate.base_snapshot, base);
    assert_eq!(
        candidate.validation,
        scriberig_project::CandidateValidation::Passed
    );
    let effect = store
        .project_effect(candidate_id)
        .expect("read effect receipt")
        .expect("durable candidate effect");
    assert_eq!(effect.project, project);
    assert_eq!(effect.base_snapshot, base);
    assert_eq!(effect.state, "applied");
    let successor = effect.successor_snapshot.expect("effect successor");
    assert!(
        store
            .parents(successor)
            .expect("successor history")
            .contains(&base)
    );
    let paths = store
        .project_effect_paths(candidate_id)
        .expect("read per-path effect receipts");
    assert_eq!(paths.len(), candidate.effects.len());
    assert!(paths.iter().all(|path| path.state == "applied"));
    successor
}

/// Verifies applied PHP source classification, declaration search, and projection.
///
/// # Parameters
/// - `checkout`: Candidate-applied checkout.
/// - `database`: Durable product database.
/// - `project`: Exact project identity.
/// - `base`: Pre-candidate immutable snapshot.
/// - `super`: Parent PTY test module.
pub(super) fn assert_applied_php(
    checkout: &std::path::Path,
    database: &std::path::Path,
    project: scriberig_core::ProjectId,
    base: scriberig_core::SnapshotId,
) {
    assert!(
        checkout
            .join("crates/domains/scriberig-domain-php/src/lib.rs")
            .is_file()
    );
    let store = Arc::new(SqliteStore::open(database).expect("open applied project store"));
    let workspace = ProjectService::new(Arc::clone(&store))
        .workspace(project)
        .expect("successor semantic workspace");
    assert_ne!(workspace.snapshot, base);
    let php_path = "tests/self-hosting/php-extension/php-project/src/Greeter.php";
    let file = store
        .project_files(project)
        .expect("PHP project manifest")
        .into_iter()
        .find(|file| file.relative_path == php_path)
        .expect("imported PHP fixture");
    assert_eq!(file.adapter, "php");
    assert_eq!(
        file.bytes,
        fs::read(checkout.join(php_path)).expect("exact PHP fixture bytes")
    );
    let service = ProjectService::new(Arc::new(
        SqliteStore::open(database).expect("reopen project store"),
    ));
    let symbols = service
        .find_symbols_at(project, workspace.snapshot, "Greeter", 20)
        .expect("find PHP declaration");
    let class = symbols
        .iter()
        .find(|symbol| symbol.name.as_deref() == Some("Greeter"))
        .expect("discover PHP class");
    assert_eq!(class.kind, "php.class");
    let method = symbols
        .iter()
        .find(|symbol| symbol.name.as_deref() == Some("greet"))
        .expect("discover PHP method");
    assert_eq!(method.kind, "php.method");
    let projection = service
        .projection_at(project, workspace.snapshot, class.entity)
        .expect("render PHP source projection");
    assert_eq!(projection.text, file.bytes);
}

/// Verifies candidate effects materialized before the updated parser is restarted.
///
/// # Parameters
/// - `checkout`: Candidate-applied copied checkout.
/// - `database`: Product database with the applied candidate and refreshed snapshot.
/// - `project`: Exact project identity.
/// - `base`: Exact candidate base snapshot.
/// - `super`: Parent PTY test module.
///
/// # Returns
/// Authoritative snapshot created by Apply before the candidate binary restart.
pub(super) fn assert_materialized_php_fixture(
    checkout: &std::path::Path,
    database: &std::path::Path,
    project: scriberig_core::ProjectId,
    base: scriberig_core::SnapshotId,
) -> scriberig_core::SnapshotId {
    assert!(
        checkout
            .join("crates/domains/scriberig-domain-php/src/lib.rs")
            .is_file()
    );
    let store = SqliteStore::open(database).expect("open applied project store");
    let workspace = ProjectService::new(Arc::new(
        SqliteStore::open(database).expect("project store"),
    ))
    .workspace(project)
    .expect("successor semantic workspace");
    assert_ne!(workspace.snapshot, base);
    let php_path = "tests/self-hosting/php-extension/php-project/src/Greeter.php";
    let file = store
        .project_files(project)
        .expect("PHP project manifest")
        .into_iter()
        .find(|file| file.relative_path == php_path)
        .expect("PHP fixture remains in refreshed manifest");
    assert_eq!(
        file.bytes,
        fs::read(checkout.join(php_path)).expect("exact PHP fixture bytes")
    );
    workspace.snapshot
}
