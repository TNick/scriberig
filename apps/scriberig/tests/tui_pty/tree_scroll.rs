//! Verifies scrolling and terminal resizing retain semantic tree selection.

use super::tui_pty_support::{TuiPty, test_lock};
use portable_pty::PtySize;
use scriberig_session::SessionStore;

/// Seeds one root with enough durable children to require tree scrolling.
///
/// - `tui`: Running process whose private database receives the fixture.
/// - `session`: Stable session identity shown by the conversation tree.
/// - `root`: Stable root branch identity.
/// - `children`: Stable branch IDs in display order.
fn seed_scroll_session(
    tui: &TuiPty,
    session: scriberig_core::EntityId,
    root: scriberig_core::EntityId,
    children: &[scriberig_core::EntityId],
) {
    let root_turn = scriberig_core::EntityId::new();
    let mut root_branch = scriberig_session::Branch::new(root, "Many-branch root")
        .expect("create scroll fixture root");
    root_branch.children = children.to_vec();
    root_branch.status = scriberig_session::BranchStatus::Completed;
    root_branch.push_turn(seeded_turn(root_turn, "Many-branch root", "root prompt"));
    let mut branches = vec![root_branch];
    branches.extend(children.iter().enumerate().map(|(index, id)| {
        let mut branch =
            scriberig_session::Branch::new(*id, format!("Many-branch child {index:02}"))
                .expect("create scroll fixture child");
        branch.parent = Some(root);
        branch.status = scriberig_session::BranchStatus::Completed;
        branch.continuation = Some(scriberig_session::ContinuationProvenance {
            anchor: scriberig_session::ContinuationAnchor::AssistantOutput {
                branch: root,
                turn: root_turn,
            },
            origin: scriberig_session::ContinuationOrigin::HumanContinuation,
        });
        branch.push_turn(seeded_turn(
            scriberig_core::EntityId::new(),
            &format!("Many-branch child {index:02}"),
            &format!("child prompt {index:02}"),
        ));
        branch
    }));
    scriberig_session_sqlite::SqliteSessionStore::open(tui.fixture_path().join("scriberig.sqlite"))
        .expect("open PTY session store")
        .save(&scriberig_session::Session {
            id: session,
            branches,
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save scroll fixture");
}

/// Creates one completed user and assistant turn for a seeded branch.
///
/// - `id`: Stable turn identity.
/// - `title`: Branch title used for the assistant response.
/// - `prompt`: Human text stored in the turn.
///
/// The completed durable turn.
/// # Returns
/// A durable assistant turn with a deterministic entity identity.
fn seeded_turn(id: scriberig_core::EntityId, title: &str, prompt: &str) -> scriberig_session::Turn {
    scriberig_session::Turn {
        id,
        ordinal: 0,
        human: Some(scriberig_session::HumanInput {
            text: prompt.to_owned(),
        }),
        agent: Some(scriberig_session::AgentOutput {
            text: format!("{title} answer"),
        }),
        tool_invocation: None,
        tool_result: None,
    }
}

/// Returns whether a named row is selected in the painted terminal screen.
///
/// - `screen`: Captured virtual terminal contents.
/// - `label`: Exact row label expected beside the selection marker.
///
/// `true` when the selected row contains `label`.
/// # Returns
/// Whether the selected row is visible in the captured terminal screen.
fn selected_row_is_visible(screen: &str, label: &str) -> bool {
    screen
        .lines()
        .any(|line| line.contains('›') && line.contains(label))
}

/// Keeps a scrolled semantic branch selected through narrow and wide resizes.
#[test]
fn tui_pty_scroll_and_resize_keep_the_selected_semantic_branch_visible() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_with_arithmetic_model();
    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    let session = scriberig_core::EntityId::new();
    let root = scriberig_core::EntityId::new();
    let children = (0..30)
        .map(|_| scriberig_core::EntityId::new())
        .collect::<Vec<_>>();
    seed_scroll_session(&tui, session, root, &children);
    tui.send_slow(format!("/switch {session}\r").as_bytes());
    assert!(
        tui.read_until("Many-branch child 29"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1b[18~");
    assert!(
        tui.read_until_next("Enter pin reply target"),
        "{}",
        tui.diagnostic()
    );
    for _ in 0..children.len() + 2 {
        if selected_row_is_visible(&tui.screen(), "Many-branch child 29") {
            break;
        }
        tui.send(b"\x1b[B");
        tui.settle();
    }
    let selected = tui.screen();
    assert!(
        selected_row_is_visible(&selected, "Many-branch child 29"),
        "the final semantic child must be selected before resize:\n{selected}"
    );

    tui.resize(PtySize {
        rows: 18,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    });
    assert!(
        tui.read_until("Many-branch child 29"),
        "{}",
        tui.diagnostic()
    );
    let narrow = tui.screen();
    assert!(
        selected_row_is_visible(&narrow, "Many-branch child 29"),
        "scrolling and narrow resize must preserve the selected semantic child:\n{narrow}"
    );
    assert!(
        !narrow.contains("Many-branch child 00"),
        "the selected tail row should scroll the earlier children out of view:\n{narrow}"
    );

    tui.resize(PtySize {
        rows: 50,
        cols: 140,
        pixel_width: 0,
        pixel_height: 0,
    });
    assert!(
        tui.read_until("Many-branch child 29"),
        "{}",
        tui.diagnostic()
    );
    let wide = tui.screen();
    assert!(
        selected_row_is_visible(&wide, "Many-branch child 29"),
        "wide resize must retain the same semantic selection:\n{wide}"
    );
    assert!(tui.shutdown());
}
