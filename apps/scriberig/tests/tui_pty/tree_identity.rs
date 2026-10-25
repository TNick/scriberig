//! Verifies recursive conversation tree identity survives visible collapse and
//! expansion, and that expansion stays a presentation-only state.

use super::tui_pty_support::{TuiPty, test_lock};
use portable_pty::PtySize;
use scriberig_session::SessionStore;

/// Durable identities for the seeded four-level conversation.
///
/// # Public members
/// - `session`: Owning session identity.
/// - `root`: Root branch identity.
/// - `child`: First nested branch identity.
/// - `grandchild`: Second nested branch identity.
/// - `leaf`: Deepest branch identity.
/// - `root_turn`: Durable turn that anchors the `child` continuation.
/// - `child_turn`: Durable turn that anchors the `grandchild` continuation.
/// - `grandchild_turn`: Durable turn that anchors the `leaf` continuation.
/// - `leaf_turn`: Durable turn owned by the `leaf` branch.
struct Fixture {
    /// Owning session identity.
    session: scriberig_core::EntityId,
    /// Root branch identity.
    root: scriberig_core::EntityId,
    /// First nested branch identity.
    child: scriberig_core::EntityId,
    /// Second nested branch identity.
    grandchild: scriberig_core::EntityId,
    /// Deepest branch identity.
    leaf: scriberig_core::EntityId,
    /// Durable turn that anchors the `child` continuation.
    root_turn: scriberig_core::EntityId,
    /// Durable turn that anchors the `grandchild` continuation.
    child_turn: scriberig_core::EntityId,
    /// Durable turn that anchors the `leaf` continuation.
    grandchild_turn: scriberig_core::EntityId,
    /// Durable turn owned by the `leaf` branch.
    leaf_turn: scriberig_core::EntityId,
}

impl Fixture {
    /// Creates fresh durable identities for one recursive conversation.
    ///
    /// The unused identity set.
    /// # Returns
    /// A new fixture identity set.
    #[must_use]
    fn new() -> Self {
        Self {
            session: scriberig_core::EntityId::new(),
            root: scriberig_core::EntityId::new(),
            child: scriberig_core::EntityId::new(),
            grandchild: scriberig_core::EntityId::new(),
            leaf: scriberig_core::EntityId::new(),
            root_turn: scriberig_core::EntityId::new(),
            child_turn: scriberig_core::EntityId::new(),
            grandchild_turn: scriberig_core::EntityId::new(),
            leaf_turn: scriberig_core::EntityId::new(),
        }
    }
}

/// One completed durable branch with one human and one assistant turn.
///
/// - `id`: Stable branch identity.
/// - `turn_id`: Stable durable turn identity inside this branch.
/// - `title`: Branch title projected into the visible tree row.
/// - `parent`: Structural parent branch, when the branch is nested.
/// - `children`: Durable child branch identities.
/// - `anchor`: Exact historical conversation point this branch continues.
/// - `question`: Human turn text rendered as a conversation row.
///
/// The seeded branch.
/// # Returns
/// A durable `Branch` value.
fn seeded_branch(
    id: scriberig_core::EntityId,
    turn_id: scriberig_core::EntityId,
    title: &str,
    parent: Option<scriberig_core::EntityId>,
    children: Vec<scriberig_core::EntityId>,
    anchor: Option<scriberig_core::EntityId>,
    question: &str,
) -> scriberig_session::Branch {
    let mut branch = scriberig_session::Branch::new(id, title).expect("create seeded branch");
    branch.parent = parent;
    branch.children = children;
    branch.status = scriberig_session::BranchStatus::Completed;
    if let (Some(anchor_branch), Some(anchor_turn)) = (parent, anchor) {
        branch.continuation = Some(scriberig_session::ContinuationProvenance {
            anchor: scriberig_session::ContinuationAnchor::AssistantOutput {
                branch: anchor_branch,
                turn: anchor_turn,
            },
            origin: scriberig_session::ContinuationOrigin::HumanContinuation,
        });
    }
    branch.push_turn(scriberig_session::Turn {
        id: turn_id,
        ordinal: 0,
        human: Some(scriberig_session::HumanInput {
            text: question.to_owned(),
        }),
        agent: Some(scriberig_session::AgentOutput {
            text: format!("{title} answer"),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    branch
}

/// Persists one four-level recursive conversation whose nested branches each
/// continue an assistant output of their structural parent.
///
/// - `tui`: Running process whose private database receives the fixture.
/// - `fixture`: Durable identities for the seeded branches and turns.
fn seed_conversation(tui: &TuiPty, fixture: &Fixture) {
    let root = seeded_branch(
        fixture.root,
        fixture.root_turn,
        "Recursive root branch",
        None,
        vec![fixture.child],
        None,
        "root branch question",
    );
    let child = seeded_branch(
        fixture.child,
        fixture.child_turn,
        "Nested child branch",
        Some(fixture.root),
        vec![fixture.grandchild],
        Some(fixture.root_turn),
        "nested child question",
    );
    let grandchild = seeded_branch(
        fixture.grandchild,
        fixture.grandchild_turn,
        "Nested grandchild branch",
        Some(fixture.child),
        vec![fixture.leaf],
        Some(fixture.child_turn),
        "nested grandchild question",
    );
    let leaf = seeded_branch(
        fixture.leaf,
        fixture.leaf_turn,
        "Deepest leaf branch",
        Some(fixture.grandchild),
        Vec::new(),
        Some(fixture.grandchild_turn),
        "deepest leaf question",
    );

    scriberig_session_sqlite::SqliteSessionStore::open(tui.fixture_path().join("scriberig.sqlite"))
        .expect("open PTY session store")
        .save(&scriberig_session::Session {
            id: fixture.session,
            branches: vec![root, child, grandchild, leaf],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("seed recursive conversation");
}

/// Returns whether `needle` is inside the row the upstream tree marks selected.
///
/// - `screen`: Captured virtual terminal screen.
/// - `needle`: Visible row label to test.
///
/// `true` when the marked selected row contains the label.
/// # Returns
/// Whether that row is the selected one.
fn is_selected(screen: &str, needle: &str) -> bool {
    screen
        .lines()
        .any(|line| line.contains('›') && line.contains(needle))
}

/// Moves the semantic cursor with `key` until the selected row contains `needle`.
///
/// - `tui`: Running process in conversation browse mode.
/// - `needle`: Visible row label to select.
/// - `key`: Terminal bytes for one upstream tree movement.
///
/// `true` when the row became selected within eight movements.
/// # Returns
/// Whether the target row was selected.
fn select_row(tui: &mut TuiPty, needle: &str, key: &[u8]) -> bool {
    for _ in 0..8 {
        if is_selected(&tui.screen(), needle) {
            return true;
        }
        tui.send(key);
        tui.settle();
    }
    is_selected(&tui.screen(), needle)
}

/// Verifies nested branch rows stay identified by their durable branch identity
/// while presentation expansion hides and reveals only their activity rows.
///
/// The PTY scenario spans startup, durable seeding, browse-mode navigation,
/// nested expansion and collapse, reply anchoring, and persistence, so its
/// end-to-end ordering stays visible together.
#[allow(clippy::too_many_lines)]
#[test]
fn tui_pty_keeps_conversation_tree_identity_across_collapse_and_expand() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_with_arithmetic_model();
    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    tui.resize(PtySize {
        rows: 50,
        cols: 140,
        pixel_width: 0,
        pixel_height: 0,
    });
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());

    let fixture = Fixture::new();
    seed_conversation(&tui, &fixture);

    tui.send_slow(format!("/switch {}\r", fixture.session).as_bytes());
    assert!(
        tui.read_until("Recursive root branch"),
        "{}",
        tui.diagnostic()
    );
    assert!(
        tui.read_until("You: root branch question"),
        "{}",
        tui.diagnostic()
    );

    let opened = tui.screen();
    for title in [
        "Nested child branch",
        "Nested grandchild branch",
        "Deepest leaf branch",
    ] {
        assert!(
            opened.contains(title),
            "every durable branch keeps its own row:\n{opened}"
        );
    }
    for question in [
        "nested child question",
        "nested grandchild question",
        "deepest leaf question",
    ] {
        assert!(
            !opened.contains(question),
            "a completed child branch starts compact without its activity rows:\n{opened}"
        );
    }

    tui.send(b"\x1b[18~");
    assert!(
        tui.read_until_next("Enter pin reply target"),
        "{}",
        tui.diagnostic()
    );

    assert!(
        select_row(&mut tui, "Nested child branch", b"\x1b[B"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1b[C");
    tui.settle();
    let expanded = tui.screen();
    assert!(
        expanded.contains("You: nested child question"),
        "expanding the selected branch reveals its activity rows:\n{expanded}"
    );
    assert!(
        is_selected(&expanded, "Nested child branch"),
        "expansion must keep the stable branch row selected:\n{expanded}"
    );

    assert!(
        select_row(&mut tui, "Nested grandchild branch", b"\x1b[B"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1b[C");
    tui.settle();
    let deepened = tui.screen();
    assert!(
        deepened.contains("You: nested grandchild question"),
        "a second nesting level expands independently:\n{deepened}"
    );

    tui.send(b"\x1b[D");
    tui.settle();
    let collapsed = tui.screen();
    assert!(
        !collapsed.contains("You: nested grandchild question"),
        "collapsing hides only the selected branch activity rows:\n{collapsed}"
    );
    assert!(
        collapsed.contains("Deepest leaf branch")
            && collapsed.contains("You: nested child question"),
        "structural descendant rows survive an ancestor collapse:\n{collapsed}"
    );
    assert!(
        is_selected(&collapsed, "Nested grandchild branch"),
        "collapse must keep the stable branch row selected:\n{collapsed}"
    );

    // The deepest leaf branch has no structural child row, so it projects no child
    // at all until the semantic reducer expands it. The retained upstream tree must
    // reveal the activity rows that expansion materializes instead of keeping the
    // pre-existing row collapsed, and must still hide them when it collapses again.
    assert!(
        select_row(&mut tui, "Deepest leaf branch", b"\x1b[B"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1b[C");
    tui.settle();
    let leaf_opened = tui.screen();
    assert!(
        leaf_opened.contains("You: deepest leaf question"),
        "expanding a completed leaf branch reveals its activity rows:\n{leaf_opened}"
    );
    assert!(
        is_selected(&leaf_opened, "Deepest leaf branch"),
        "leaf expansion must keep the stable branch row selected:\n{leaf_opened}"
    );
    tui.send(b"\x1b[D");
    tui.settle();
    let leaf_closed = tui.screen();
    assert!(
        !leaf_closed.contains("You: deepest leaf question"),
        "collapsing the leaf branch again hides only its own activity rows:\n{leaf_closed}"
    );
    assert!(
        leaf_closed.contains("Nested grandchild branch")
            && leaf_closed.contains("You: nested child question"),
        "an ancestor collapse and its sibling rows survive a leaf collapse:\n{leaf_closed}"
    );

    assert!(
        select_row(&mut tui, "Nested child branch", b"\x1b[A"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\r");
    assert!(
        tui.read_until("Reply target pinned"),
        "{}",
        tui.diagnostic()
    );

    let loaded = tui.load_session(fixture.session);
    let durable = |id: scriberig_core::EntityId| -> scriberig_session::Branch {
        loaded
            .branches
            .iter()
            .find(|item| item.id == id)
            .expect("durable branch survives presentation state")
            .clone()
    };
    assert_eq!(durable(fixture.child).parent, Some(fixture.root));
    assert_eq!(durable(fixture.grandchild).parent, Some(fixture.child));
    assert_eq!(durable(fixture.leaf).parent, Some(fixture.grandchild));
    for id in [fixture.child, fixture.grandchild, fixture.leaf] {
        assert_eq!(
            durable(id).turns.len(),
            1,
            "collapse and expand must not change durable activity"
        );
    }
    assert_eq!(loaded.branches.len(), 4);
    assert!(tui.shutdown());
}
