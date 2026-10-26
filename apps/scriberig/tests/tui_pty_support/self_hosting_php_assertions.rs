//! Verifies durable PHP candidate application and semantic restart evidence.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use super::TuiPty;
use super::candidate_review::{
    assert_confirmation_closed, confirm_candidate_apply, open_apply_confirmation,
};
use scriberig_app::{ApplicationContext, HistoryRequest, OperationProvider, SnapshotSelector};
use scriberig_project::ProjectService;
use scriberig_project::ProjectSourceEffectKind;
use scriberig_store::SnapshotStore;
use scriberig_store_sqlite::SqliteStore;
use std::time::Duration;

/// Verifies tracked Serena configuration is copied without local overrides.
///
/// - `checkout`: Isolated copied checkout.
/// - `super`: Parent test module visibility marker.
///
/// # Panics
/// Panics when tracked Serena project configuration differs from the source.
pub(super) fn assert_tracked_serena_configuration(checkout: &Path) {
    let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = repository.join(".serena/project.yml");
    if source.is_file() {
        assert_eq!(
            fs::read(checkout.join(".serena/project.yml"))
                .expect("tracked Serena configuration in copied checkout"),
            fs::read(source).expect("source Serena configuration")
        );
    }
}

/// Verifies the applied snapshot through the shared exact history operation.
///
/// - `database`: Durable product database path.
/// - `base`: Exact snapshot before candidate Apply.
/// - `successor`: Authoritative snapshot after candidate Apply.
/// - `candidate`: Accepted project candidate identity.
/// - `super`: Parent PTY test module visibility.
pub(super) fn assert_applied_history_transition(
    database: &Path,
    base: scriberig_core::SnapshotId,
    successor: scriberig_core::SnapshotId,
    candidate: &str,
) {
    let history_summary = scriberig_app::StoreOperationProvider::new(Arc::new(
        SqliteStore::open(database).expect("history database"),
    ))
    .history(
        &ApplicationContext::empty(),
        &HistoryRequest {
            snapshot: SnapshotSelector::Exact(successor),
        },
    )
    .expect("shared exact history operation");
    assert!(history_summary.history.as_deref().is_some_and(|entries| {
        entries.iter().any(|entry| {
            entry.base == base
                && entry.result == successor
                && entry.source_change_id.as_deref() == Some(candidate)
        })
    }));
}

/// Verifies Apply leaves an unrelated repository source file unchanged.
///
/// - `checkout`: Copied repository after candidate application.
/// - `before`: Exact unrelated source bytes captured before proposal.
/// - `super`: Parent test module visibility.
pub(super) fn assert_unrelated_source_unchanged(checkout: &std::path::Path, before: &[u8]) {
    let source = checkout.join("crates/scriberig-query/src/lib.rs");
    assert_eq!(
        fs::read(&source).expect("read unrelated source after Apply"),
        before,
        "project-wide candidate changed unrelated source {}",
        source.display()
    );
}

/// Exercises visible update/create conflicts before the candidate's successful Apply.
///
/// - `tui`: Active native TUI process.
/// - `checkout`: Copied Scriberig checkout.
/// - `database`: Product database path.
/// - `project`: Onboarded project identity.
/// - `base`: Exact candidate base snapshot.
/// - `candidate_id`: Durable project-wide candidate identity.
/// - `super`: Parent PTY test module.
pub(super) fn assert_candidate_conflicts_are_visible(
    tui: &mut TuiPty,
    checkout: &std::path::Path,
    database: &std::path::Path,
    project: scriberig_core::ProjectId,
    base: scriberig_core::SnapshotId,
    candidate_id: &str,
) {
    let candidate = ProjectService::new(Arc::new(SqliteStore::open(database).expect("store")))
        .candidate(candidate_id)
        .expect("load candidate")
        .expect("durable candidate");
    let create = candidate
        .effects
        .iter()
        .find(|effect| {
            effect.kind == ProjectSourceEffectKind::Create
                && effect
                    .path
                    .starts_with("crates/domains/scriberig-domain-php/")
        })
        .expect("PHP adapter create effect");
    let create_path = checkout.join(&create.path);
    fs::create_dir_all(create_path.parent().expect("create parent"))
        .expect("create external path parent");
    let external_create = b"external bytes must survive Apply conflict\n";
    open_apply_confirmation(tui);
    fs::write(&create_path, external_create).expect("external create");
    confirm_candidate_apply(tui);
    assert!(
        tui.read_until_for("filesystem_conflict", Duration::from_secs(30)),
        "{}",
        tui.diagnostic()
    );
    assert_confirmation_closed(tui);
    assert!(tui.screen().contains("source changed externally:"));
    assert_eq!(
        fs::read(&create_path).expect("external create survives"),
        external_create
    );
    assert_candidate_base_unchanged(database, project, base);
    assert!(
        !checkout
            .join("crates/domains/scriberig-domain-php/src/lib.rs")
            .exists()
    );
    fs::remove_dir_all(checkout.join("crates/domains/scriberig-domain-php"))
        .expect("remove external conflict fixture");

    reopen_candidate_review(tui);
    open_apply_confirmation(tui);
    let manifest_path = checkout.join("Cargo.toml");
    let base_manifest = fs::read(&manifest_path).expect("base manifest");
    let external_update = b"external manifest edit\n";
    fs::write(&manifest_path, external_update).expect("external update");
    confirm_candidate_apply(tui);
    assert!(
        tui.read_until_for("filesystem_conflict", Duration::from_secs(30)),
        "{}",
        tui.diagnostic()
    );
    assert_confirmation_closed(tui);
    assert!(tui.screen().contains("source changed externally:"));
    assert_eq!(
        fs::read(&manifest_path).expect("external update survives"),
        external_update
    );
    assert_candidate_base_unchanged(database, project, base);
    assert!(
        !checkout
            .join("crates/domains/scriberig-domain-php/src/lib.rs")
            .exists()
    );
    fs::write(&manifest_path, base_manifest).expect("restore exact base manifest");
    reopen_candidate_review(tui);
}

/// Opens the durable project candidate review with a local command.
///
/// - `tui`: Active native TUI process.
fn reopen_candidate_review(tui: &mut TuiPty) {
    tui.send(b"\x10");
    assert!(
        tui.read_until_for("Command palette", Duration::from_secs(120)),
        "command palette did not open for candidate review: {}",
        tui.diagnostic()
    );
    tui.send_slow(b"Review candidate");
    tui.send(b"\r");
    assert!(
        tui.read_until_not("operation_failed"),
        "candidate review did not clear its previous operation error: {}",
        tui.diagnostic()
    );
    assert!(
        tui.read_until("Validation: Passed"),
        "candidate review did not finish refreshing: {}",
        tui.diagnostic()
    );
}

/// Verifies a conflict leaves the authoritative project at its exact base.
///
/// - `database`: Product database path.
/// - `project`: Onboarded project identity.
/// - `base`: Exact candidate base snapshot.
fn assert_candidate_base_unchanged(
    database: &std::path::Path,
    project: scriberig_core::ProjectId,
    base: scriberig_core::SnapshotId,
) {
    let workspace = ProjectService::new(Arc::new(SqliteStore::open(database).expect("store")))
        .workspace(project)
        .expect("authoritative project workspace");
    assert_eq!(workspace.snapshot, base);
}

/// Verifies durable candidate validation, application receipts, and snapshot lineage.
///
/// - `database`: Reopened product database.
/// - `project`: Exact project identity.
/// - `base`: Candidate base snapshot.
/// - `candidate_id`: Candidate identity returned by the root agent result.
/// - `super`: Parent PTY test module.
///
/// Applied successor snapshot recorded by the durable effect receipt.
/// # Returns
/// The operation result.
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
    assert_eq!(
        candidate.validation_profile.as_deref(),
        Some("project-ingestion-e2e")
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
/// - `checkout`: Candidate-applied copied checkout.
/// - `database`: Product database with the applied candidate and refreshed snapshot.
/// - `project`: Exact project identity.
/// - `base`: Exact candidate base snapshot.
/// - `super`: Parent PTY test module.
///
/// Authoritative snapshot created by Apply before the candidate binary restart.
/// # Returns
/// The operation result.
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
