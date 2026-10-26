//! Verifies semantic snapshot diff navigation through the native TUI.

use std::sync::Arc;

use scriberig_project::ProjectService;
use scriberig_store::{Revision, RevisionStore, SnapshotChange, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

use super::tui_pty_support::{TuiPty, test_lock};

/// Opens an exact semantic snapshot diff through the product command flow.
#[test]
fn tui_pty_opens_semantic_diff_between_exact_snapshots() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_golden();
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());

    let database = tui.fixture_path().join("scriberig.sqlite");
    let store = Arc::new(SqliteStore::open(database).expect("open fixture store"));
    let project = store
        .list_projects()
        .expect("list imported projects")
        .into_iter()
        .next()
        .expect("imported project");
    let base = ProjectService::new(Arc::clone(&store))
        .workspace(project.id)
        .expect("resolve semantic workspace")
        .snapshot;
    let (entity, revision_id) = store
        .iter(base)
        .expect("read base snapshot")
        .into_iter()
        .next()
        .expect("base snapshot contains a semantic entity");
    let revision = store
        .get(revision_id)
        .expect("read base revision")
        .expect("base revision is retained");
    let mut payload = revision.payload().to_vec();
    payload.extend_from_slice(b" semantic diff fixture");
    let changed_revision =
        Revision::new(revision.version(), revision.object_type().clone(), payload)
            .expect("build changed immutable revision");
    let changed_revision_id = store
        .put(changed_revision)
        .expect("persist changed revision");
    let target = store
        .apply_changes(base, &[SnapshotChange::Set(entity, changed_revision_id)])
        .expect("persist target snapshot");

    tui.send_slow(b"/diff\r");
    assert!(
        tui.read_until("Choose diff snapshot"),
        "{}",
        tui.diagnostic()
    );
    tui.send_slow(format!("{target}\r").as_bytes());
    assert!(
        tui.read_until(&format!("Target snapshot: {target}")),
        "{}",
        tui.diagnostic()
    );
    let screen = tui.screen();
    assert!(
        screen.contains(&format!("Base snapshot: {base}")),
        "{}",
        tui.diagnostic()
    );
    assert!(screen.contains(&entity.to_string()), "{}", tui.diagnostic());
    assert!(
        screen.contains(&revision_id.to_string()),
        "{}",
        tui.diagnostic()
    );
    assert!(
        screen.contains(&changed_revision_id.to_string()),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.shutdown());
}
