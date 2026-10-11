//! Projects provider-neutral sessions into stable terminal tree rows.

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
        for branch in SessionViews::branches(session) {
            let transient = branch.status != BranchStatus::Completed;
            rows.push(SessionRow {
                reference: branch.id,
                depth: 1,
                label: format!("{} [{:?}]", branch.title, branch.status),
                kind: SessionRowKind::Branch,
                transient,
            });
            if let Some(source) = session.branches.iter().find(|item| item.id == branch.id) {
                for turn in SessionViews::turns(source) {
                    rows.push(SessionRow {
                        reference: turn.id,
                        depth: 2,
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
                            depth: 3,
                            label: format!("Tool {reference}"),
                            kind: SessionRowKind::Tool,
                            transient,
                        });
                    }
                }
                if let Some(result) = source.result.as_ref() {
                    rows.push(SessionRow {
                        reference: result.id,
                        depth: 2,
                        label: "Durable branch result".to_owned(),
                        kind: SessionRowKind::Result,
                        transient: false,
                    });
                    rows.extend(result.references.iter().map(|reference| SessionRow {
                        reference: *reference,
                        depth: 3,
                        label: format!("Result reference {reference}"),
                        kind: SessionRowKind::Result,
                        transient: false,
                    }));
                }
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

#[cfg(test)]
mod tests {
    use super::{SessionModel, SessionRowKind};
    use scriberig_core::EntityId;
    use scriberig_session::{Branch, Session, Turn};

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
}
