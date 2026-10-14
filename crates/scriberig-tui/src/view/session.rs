//! Projects provider-neutral sessions into stable terminal tree rows.

use std::collections::BTreeSet;

use scriberig_core::EntityId;
use scriberig_session::{BranchStatus, Session, SessionViews};

/// One semantic row in the session tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionRow {
    /// Stable semantic object represented by the row.
    pub reference: EntityId,
    /// Nesting depth in the session tree.
    pub depth: usize,
    /// Human-readable row label.
    pub label: String,
    /// Public session object kind.
    pub kind: SessionRowKind,
    /// Candidate handle when this row exposes a review action.
    pub candidate: Option<String>,
    /// Exact candidate base snapshot for review actions.
    pub snapshot: Option<scriberig_core::SnapshotId>,
    /// Whether the row belongs to an exploratory/non-canonical branch.
    pub transient: bool,
}

/// Classifies a session tree row without embedding widget state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionRowKind {
    /// The session root.
    Session,
    /// A session branch.
    Branch,
    /// A branch turn.
    Turn,
    /// A durable result reference produced by a branch.
    Result,
    /// A tool invocation or result reference attached to a turn.
    Tool,
    /// A durable candidate handle that can be opened for review.
    Candidate,
}

/// Cached session tree presentation model.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SessionModel {
    /// Flattened, deterministically ordered semantic rows.
    pub rows: Vec<SessionRow>,
}

impl SessionModel {
    /// Builds a tree model from the provider-neutral session value.
    ///
    /// # Parameters
    /// - `session`: Session value to project.
    ///
    /// # Returns
    /// A flattened terminal session model.
    #[must_use]
    pub fn from_session(session: &Session) -> Self {
        let mut rows = vec![SessionRow {
            reference: session.id,
            depth: 0,
            label: format!("Session {}", session.id),
            kind: SessionRowKind::Session,
            candidate: None,
            snapshot: None,
            transient: false,
        }];
        let mut visited = BTreeSet::new();
        let mut roots = session
            .branches
            .iter()
            .filter(|branch| branch.parent.is_none())
            .map(|branch| branch.id)
            .collect::<Vec<_>>();
        roots.sort_unstable();
        for branch in roots {
            append_branch(session, branch, 1, &mut visited, &mut rows);
        }
        for branch in SessionViews::branches(session) {
            if !visited.contains(&branch.id) {
                append_branch(session, branch.id, 1, &mut visited, &mut rows);
            }
        }
        Self { rows }
    }

    /// Returns the stable semantic reference at a cursor position.
    ///
    /// # Parameters
    /// - `cursor`: Row index to resolve.
    ///
    /// # Returns
    /// The row reference, when the cursor is in range.
    #[must_use]
    pub fn reference_at(&self, cursor: usize) -> Option<EntityId> {
        self.rows.get(cursor).map(|row| row.reference)
    }
}

/// Appends one branch and its nested activity in deterministic order.
///
/// # Parameters
/// - `session`: Session graph to traverse.
/// - `branch_id`: Branch identity to append.
/// - `depth`: Current tree depth.
/// - `visited`: Branches already traversed.
/// - `rows`: Output rows being built.
fn append_branch(
    session: &Session,
    branch_id: EntityId,
    depth: usize,
    visited: &mut BTreeSet<EntityId>,
    rows: &mut Vec<SessionRow>,
) {
    if !visited.insert(branch_id) {
        return;
    }
    let Some(source) = session
        .branches
        .iter()
        .find(|branch| branch.id == branch_id)
    else {
        return;
    };
    let transient = source.status != BranchStatus::Completed;
    rows.push(SessionRow {
        reference: source.id,
        depth,
        label: format!("{} [{:?}]", source.title, source.status),
        kind: SessionRowKind::Branch,
        candidate: None,
        snapshot: None,
        transient,
    });
    let compact = source.parent.is_some() && source.status == BranchStatus::Completed;
    if !compact {
        append_turns(source, depth, transient, rows);
    }
    append_result(source, depth, rows);
    append_children(session, source, depth, visited, rows);
}

/// Appends turn and tool rows for one branch.
///
/// # Parameters
/// - `source`: Branch whose activity is projected.
/// - `depth`: Branch row depth.
/// - `transient`: Whether the branch is still active.
/// - `rows`: Output rows being built.
fn append_turns(
    source: &scriberig_session::Branch,
    depth: usize,
    transient: bool,
    rows: &mut Vec<SessionRow>,
) {
    for turn in SessionViews::turns(source) {
        let agent_output = source
            .turns
            .iter()
            .find(|stored| stored.id == turn.id)
            .and_then(|stored| stored.agent.as_ref());
        let label = agent_output.map_or_else(
            || format!("Turn {}", turn.ordinal),
            |agent| {
                format!(
                    "Turn {}: {}",
                    turn.ordinal,
                    crate::inspector::sanitize(&agent.text)
                )
            },
        );
        rows.push(SessionRow {
            reference: turn.id,
            depth: depth + 1,
            label,
            kind: SessionRowKind::Turn,
            candidate: None,
            snapshot: None,
            transient,
        });
        append_turn_tools(
            turn.tool_invocation,
            turn.tool_result,
            depth,
            transient,
            rows,
        );
    }
}

/// Appends tool invocation and result rows for one turn.
///
/// # Parameters
/// - `invocation`: Stable tool invocation referenced by the turn, when present.
/// - `result`: Stable tool result referenced by the turn, when present.
/// - `depth`: Parent branch row depth.
/// - `transient`: Whether the owning branch is still active.
/// - `rows`: Output rows being built.
fn append_turn_tools(
    invocation: Option<EntityId>,
    result: Option<EntityId>,
    depth: usize,
    transient: bool,
    rows: &mut Vec<SessionRow>,
) {
    for reference in [invocation, result].into_iter().flatten() {
        rows.push(SessionRow {
            reference,
            depth: depth + 2,
            label: format!("Tool {reference}"),
            kind: SessionRowKind::Tool,
            candidate: None,
            snapshot: None,
            transient,
        });
    }
}

/// Appends durable result, reference, and candidate rows for one branch.
///
/// # Parameters
/// - `source`: Branch whose durable result is projected.
/// - `depth`: Branch row depth.
/// - `rows`: Output rows being built.
fn append_result(source: &scriberig_session::Branch, depth: usize, rows: &mut Vec<SessionRow>) {
    if let Some(result) = source.result.as_ref() {
        rows.push(SessionRow {
            reference: result.id,
            depth: depth + 1,
            label: "Durable branch result".to_owned(),
            kind: SessionRowKind::Result,
            candidate: None,
            snapshot: None,
            transient: false,
        });
        rows.extend(result.references.iter().map(|reference| SessionRow {
            reference: *reference,
            depth: depth + 2,
            label: format!("Result reference {reference}"),
            kind: SessionRowKind::Result,
            candidate: None,
            snapshot: None,
            transient: false,
        }));
        let base = result
            .result_handles
            .iter()
            .find_map(|handle| handle.strip_prefix("snapshot:"))
            .and_then(|snapshot| snapshot.parse().ok());
        rows.extend(result.result_handles.iter().filter_map(|handle| {
            let candidate = handle.strip_prefix("candidate:")?;
            Some(SessionRow {
                reference: result.id,
                depth: depth + 2,
                label: format!("Review candidate {candidate}"),
                kind: SessionRowKind::Candidate,
                candidate: Some(candidate.to_owned()),
                snapshot: base,
                transient: false,
            })
        }));
    }
}

/// Recursively appends child branches in stable identity order.
///
/// # Parameters
/// - `session`: Session graph containing child branches.
/// - `source`: Parent branch whose children are traversed.
/// - `depth`: Parent branch row depth.
/// - `visited`: Branches already traversed.
/// - `rows`: Output rows being built.
fn append_children(
    session: &Session,
    source: &scriberig_session::Branch,
    depth: usize,
    visited: &mut BTreeSet<EntityId>,
    rows: &mut Vec<SessionRow>,
) {
    let branch_id = source.id;
    let mut children = source.children.clone();
    if children.is_empty() {
        children = session
            .branches
            .iter()
            .filter(|branch| branch.parent == Some(branch_id))
            .map(|branch| branch.id)
            .collect();
    }
    children.sort_unstable();
    for child in children {
        append_branch(session, child, depth + 1, visited, rows);
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
