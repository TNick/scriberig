//! Verifies durable plan structure and dependency edges stay visibly separate.

use super::tui_pty_support::{TuiPty, test_lock};
use portable_pty::PtySize;
use scriberig_session::SessionStore;

/// Durable identities for the seeded plan-relation fixture.
///
/// # Public members
/// - `session`: Owning session identity.
/// - `plan`: Durable plan identity.
/// - `root_branch`: Branch that anchors the plan and parents the root nodes.
/// - `prerequisite`: Node whose result the dependency edge requires.
/// - `dependent`: Node that must wait for that result.
/// - `nested`: Node whose structural parent is `dependent`.
/// - `parentless`: Node without an explicit structural parent.
struct Fixture {
    /// Owning session identity.
    session: scriberig_core::EntityId,
    /// Durable plan identity.
    plan: scriberig_core::EntityId,
    /// Branch that anchors the plan and parents the root nodes.
    root_branch: scriberig_core::EntityId,
    /// Node whose result the dependency edge requires.
    prerequisite: scriberig_core::EntityId,
    /// Node that must wait for that result.
    dependent: scriberig_core::EntityId,
    /// Node whose structural parent is `dependent`.
    nested: scriberig_core::EntityId,
    /// Node without an explicit structural parent.
    parentless: scriberig_core::EntityId,
}

impl Fixture {
    /// Creates fresh durable identities for one plan-relation scenario.
    ///
    /// The unused identity set.
    /// # Returns
    /// A new fixture identity set.
    #[must_use]
    fn new() -> Self {
        Self {
            session: scriberig_core::EntityId::new(),
            plan: scriberig_core::EntityId::new(),
            root_branch: scriberig_core::EntityId::new(),
            prerequisite: scriberig_core::EntityId::new(),
            dependent: scriberig_core::EntityId::new(),
            nested: scriberig_core::EntityId::new(),
            parentless: scriberig_core::EntityId::new(),
        }
    }
}

/// One durable plan node for the seeded relation fixture.
///
/// - `id`: Stable plan-node identity.
/// - `parent`: Durable structural parent, never a dependency edge.
/// - `title`: Work title projected into the visible tree row.
///
/// The seeded plan node.
/// # Returns
/// A durable `PlanNode` value.
fn plan_node(
    id: scriberig_core::EntityId,
    parent: Option<scriberig_session::PlanNodeParent>,
    title: &str,
) -> scriberig_session::PlanNode {
    scriberig_session::PlanNode {
        id,
        parent,
        title: title.to_owned(),
        purpose: "Keep structural parentage separate from dependency edges".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: scriberig_session::PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
        abandon_reason: None,
        last_failure: None,
    }
}

/// Persists the plan whose structural parents and dependency edge disagree with
/// their durable display order.
///
/// - `tui`: Running process whose private database receives the fixture.
/// - `fixture`: Durable identities for the seeded plan and nodes.
fn seed_plan(tui: &TuiPty, fixture: &Fixture) {
    let mut root =
        scriberig_session::Branch::new(fixture.root_branch, "Plan relation conversation")
            .expect("create seeded conversation");
    root.status = scriberig_session::BranchStatus::Completed;

    let mut plan = scriberig_session::Plan::new(
        fixture.plan,
        fixture.session,
        scriberig_session::ContinuationAnchor::BranchHead {
            branch: fixture.root_branch,
        },
        "Structural and dependency separation",
    )
    .expect("create seeded plan");
    // The dependent node precedes its prerequisite in the durable node vector and
    // the nested node names the dependent as its structural parent, so a
    // projection that read the dependency edge or the vector order as parentage
    // would nest these rows differently from `PlanNodeParent`.
    plan.add_node(plan_node(
        fixture.dependent,
        Some(scriberig_session::PlanNodeParent::Branch(
            fixture.root_branch,
        )),
        "Dependent work item",
    ))
    .expect("add dependent node");
    plan.add_node(plan_node(
        fixture.prerequisite,
        Some(scriberig_session::PlanNodeParent::Branch(
            fixture.root_branch,
        )),
        "Prerequisite work item",
    ))
    .expect("add prerequisite node");
    plan.add_node(plan_node(
        fixture.nested,
        Some(scriberig_session::PlanNodeParent::Node(fixture.dependent)),
        "Nested structural child",
    ))
    .expect("add nested node");
    plan.add_node(plan_node(
        fixture.parentless,
        None,
        "Unparented structural root",
    ))
    .expect("add node without an explicit parent");
    plan.dependencies.push(scriberig_session::PlanDependency {
        prerequisite: fixture.prerequisite,
        dependent: fixture.dependent,
        fulfilled_by: None,
    });
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("mark seeded plan planned");

    scriberig_session_sqlite::SqliteSessionStore::open(tui.fixture_path().join("scriberig.sqlite"))
        .expect("open PTY session store")
        .save(&scriberig_session::Session {
            id: fixture.session,
            branches: vec![root],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("seed plan relations");
}

/// Returns the zero-based display column at which `needle` starts on the first
/// visible screen line that contains it.
///
/// - `screen`: Captured virtual terminal screen.
/// - `needle`: Visible row label to locate.
///
/// The character column of the label on its first matching line.
/// # Returns
/// Label column within the rendered row.
/// # Panics
/// Panics when no visible line contains the label.
fn label_column(screen: &str, needle: &str) -> usize {
    let line = screen
        .lines()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("visible row {needle} is missing from:\n{screen}"));
    line.split(needle)
        .next()
        .expect("label prefix")
        .chars()
        .count()
}

/// Returns the zero-based visible line holding the first occurrence of `needle`.
///
/// - `screen`: Captured virtual terminal screen.
/// - `needle`: Visible row label to locate.
///
/// The line position of the first matching line.
/// # Returns
/// Line position on the rendered screen.
/// # Panics
/// Panics when no visible line contains the label.
fn label_line(screen: &str, needle: &str) -> usize {
    screen
        .lines()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("visible row {needle} is missing from:\n{screen}"))
}

/// Verifies the visible plan tree nests only structural parents while the
/// explicit `PlanDependency` edge is presented as plan knowledge in the details.
///
/// The PTY scenario spans startup, durable seeding, view switching, visible tree
/// assertions, node selection, and persistence, so its end-to-end ordering stays
/// visible together.
#[allow(clippy::too_many_lines)]
#[test]
fn tui_pty_keeps_plan_dependency_edges_out_of_the_tree_structure() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_with_arithmetic_model();
    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    // The selected plan details list every durable plan revision, and the tree
    // rows render above those details, so this scenario needs a taller virtual
    // terminal to keep the whole Plans frame inside one visible page.
    tui.resize(PtySize {
        rows: 60,
        cols: 140,
        pixel_width: 0,
        pixel_height: 0,
    });
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());

    let fixture = Fixture::new();
    seed_plan(&tui, &fixture);

    tui.send_slow(format!("/switch {}\r", fixture.session).as_bytes());
    assert!(
        tui.read_until("Plan relation conversation"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x10");
    assert!(tui.read_until("Command palette"), "{}", tui.diagnostic());
    tui.send_slow(b"plans\r");
    assert!(
        tui.read_until("Plan · Structural and dependency separation"),
        "{}",
        tui.diagnostic()
    );
    assert!(
        tui.read_until(&format!(
            "{} → {} · result pending",
            fixture.prerequisite, fixture.dependent
        )),
        "{}",
        tui.diagnostic()
    );

    let tree = tui.screen();
    let header_column = label_column(&tree, "Plan · Structural and dependency separation");
    let dependent_column = label_column(&tree, "Dependent work item");
    let prerequisite_column = label_column(&tree, "Prerequisite work item");
    let parentless_column = label_column(&tree, "Unparented structural root");
    let nested_column = label_column(&tree, "Nested structural child");
    assert!(
        dependent_column == prerequisite_column,
        "a dependency edge must not nest the dependent node beneath its prerequisite:\n{tree}"
    );
    assert!(
        dependent_column > header_column
            && parentless_column == dependent_column
            && nested_column > dependent_column,
        "structural parentage from `PlanNodeParent` must stay visible:\n{tree}"
    );
    assert!(
        label_line(&tree, "Dependent work item") < label_line(&tree, "Prerequisite work item"),
        "the durable node vector must not change structural nesting:\n{tree}"
    );
    assert!(
        tree.contains(&format!("node parent {}", fixture.dependent))
            && tree.contains(&format!("branch parent {}", fixture.root_branch)),
        "each tree row must carry its own structural parent reference:\n{tree}"
    );

    // One Down selects the dependent node row. Its details must report the
    // structural parent taken from `PlanNodeParent` even though the only edge
    // names a different prerequisite node, and the tree nesting must not change.
    tui.send(b"\x1b[18~");
    tui.settle();
    tui.send(b"\x1b[B");
    assert!(
        tui.read_until(&format!(
            "Structural parent branch: {}",
            fixture.root_branch
        )),
        "{}",
        tui.diagnostic()
    );
    let selected = tui.screen();
    assert!(
        !selected.contains("Structural parent node"),
        "the dependent node has no structural parent node:\n{selected}"
    );
    assert!(
        label_column(&selected, "Nested structural child")
            > label_column(&selected, "Prerequisite work item"),
        "selecting a node must not re-parent tree rows:\n{selected}"
    );

    let loaded = tui.load_session(fixture.session);
    let loaded_plan = loaded
        .plans
        .iter()
        .find(|plan| plan.id == fixture.plan)
        .expect("durable plan survives the visible projection");
    assert_eq!(
        loaded_plan.dependencies,
        vec![scriberig_session::PlanDependency {
            prerequisite: fixture.prerequisite,
            dependent: fixture.dependent,
            fulfilled_by: None,
        }],
        "the durable dependency edge must be unchanged by presentation"
    );
    assert_eq!(
        loaded_plan
            .nodes
            .iter()
            .map(|node| (node.id, node.parent))
            .collect::<Vec<_>>(),
        vec![
            (
                fixture.dependent,
                Some(scriberig_session::PlanNodeParent::Branch(
                    fixture.root_branch
                ))
            ),
            (
                fixture.prerequisite,
                Some(scriberig_session::PlanNodeParent::Branch(
                    fixture.root_branch
                ))
            ),
            (
                fixture.nested,
                Some(scriberig_session::PlanNodeParent::Node(fixture.dependent))
            ),
            (fixture.parentless, None),
        ],
        "the durable structural parents must be unchanged by presentation"
    );
    assert!(tui.shutdown());
}
