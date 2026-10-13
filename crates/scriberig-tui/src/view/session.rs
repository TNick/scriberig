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
        transient,
    });
    let compact = source.parent.is_some() && source.status == BranchStatus::Completed;
    if !compact {
        for turn in SessionViews::turns(source) {
            rows.push(SessionRow {
                reference: turn.id,
                depth: depth + 1,
                label: format!("Turn {}", turn.ordinal),
                kind: SessionRowKind::Turn,
                transient,
            });
            for reference in [turn.tool_invocation, turn.tool_result]
                .into_iter()
                .flatten()
            {
                rows.push(SessionRow {
                    reference,
                    depth: depth + 2,
                    label: format!("Tool {reference}"),
                    kind: SessionRowKind::Tool,
                    transient,
                });
            }
        }
    }
    if let Some(result) = source.result.as_ref() {
        rows.push(SessionRow {
            reference: result.id,
            depth: depth + 1,
            label: "Durable branch result".to_owned(),
            kind: SessionRowKind::Result,
            transient: false,
        });
        rows.extend(result.references.iter().map(|reference| SessionRow {
            reference: *reference,
            depth: depth + 2,
            label: format!("Result reference {reference}"),
            kind: SessionRowKind::Result,
            transient: false,
        }));
    }
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
mod tests {
    use super::{SessionModel, SessionRowKind};
    use scriberig_core::EntityId;
    use scriberig_session::{Branch, BranchStatus, Session, Turn};

    /// Verifies branches and turns retain their semantic nesting and identity.
    #[test]
    fn builds_nested_session_rows() {
        let branch_id = EntityId::from_bytes([2; 16]);
        let mut branch = Branch::new(branch_id, "exploration").expect("branch");
        branch.push_turn(Turn {
            id: EntityId::from_bytes([3; 16]),
            ordinal: 0,
            human: None,
            agent: None,
            tool_invocation: None,
            tool_result: None,
        });
        let session = Session {
            id: EntityId::from_bytes([1; 16]),
            branches: vec![branch],
            goals: Vec::new(),
            tasks: Vec::new(),
        };
        let model = SessionModel::from_session(&session);
        assert_eq!(model.rows[0].kind, SessionRowKind::Session);
        assert_eq!(model.rows[1].depth, 1);
        assert_eq!(model.rows[2].depth, 2);
        assert_eq!(model.reference_at(2), Some(EntityId::from_bytes([3; 16])));
        assert!(model.rows[1].transient);
    }

    /// Verifies completed child branches collapse to result references.
    #[test]
    fn collapses_completed_child_activity() {
        let parent_id = EntityId::from_bytes([8; 16]);
        let child_id = EntityId::from_bytes([9; 16]);
        let turn_id = EntityId::from_bytes([10; 16]);
        let result_id = EntityId::from_bytes([11; 16]);
        let parent = Branch::new(parent_id, "parent").expect("parent");
        let mut child = Branch::new(child_id, "child").expect("child");
        child.parent = Some(parent_id);
        child.turns.push(Turn {
            id: turn_id,
            ordinal: 0,
            human: None,
            agent: None,
            tool_invocation: None,
            tool_result: None,
        });
        child.status = BranchStatus::Completed;
        child.result = Some(scriberig_session::BranchResult {
            id: result_id,
            branch: child_id,
            status: BranchStatus::Completed,
            summary: Some("done".to_owned()),
            result_kind: Some("agent".to_owned()),
            references: vec![result_id],
            result_handles: Vec::new(),
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        });
        let session = Session {
            id: EntityId::from_bytes([12; 16]),
            branches: vec![parent, child],
            goals: Vec::new(),
            tasks: Vec::new(),
        };
        let model = SessionModel::from_session(&session);
        assert!(model.rows.iter().any(|row| row.reference == result_id));
        assert!(!model.rows.iter().any(|row| row.reference == turn_id));
    }
}
