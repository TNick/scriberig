//! Projects provider-neutral sessions into stable terminal tree rows.

use std::collections::BTreeSet;

use scriberig_core::EntityId;
use scriberig_session::{BranchStatus, Session, SessionViews};

#[path = "session_activity.rs"]
mod session_activity;
#[path = "session_turns.rs"]
mod session_turns;

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
    /// A durable model invocation.
    ModelInvocation,
    /// An externally meaningful model output.
    ModelOutput,
    /// A tool invocation in the causal activity sequence.
    ToolInvocation,
    /// A tool result in the causal activity sequence.
    ToolResult,
    /// A durable candidate handle that can be opened for review.
    Candidate,
}

/// Cached session tree presentation model.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SessionModel {
    /// Flattened, deterministically ordered semantic rows.
    pub rows: Vec<SessionRow>,
    /// Shared future-invocation model and thinking selection.
    pub execution: scriberig_session::SessionExecutionSelection,
    /// Bounded schema-redacted arguments by stable tool invocation identity.
    pub tool_invocation_details: std::collections::BTreeMap<EntityId, String>,
}

impl SessionModel {
    /// Builds a tree model from the provider-neutral session value.
    ///
    /// - `session`: Session value to project.
    ///
    /// A flattened terminal session model.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn from_session(session: &Session) -> Self {
        Self::from_session_with_expanded(session, &BTreeSet::new())
    }

    /// Builds a tree model with selected completed branches expanded.
    ///
    /// Expansion is a presentation input only and does not mutate the session.
    ///
    /// - `session`: Authoritative session value to project.
    /// - `expanded`: Stable branch identities whose retained history is visible.
    ///
    /// A flattened session model with the selected completed children expanded.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn from_session_with_expanded(session: &Session, expanded: &BTreeSet<EntityId>) -> Self {
        let model_selection = execution_label(&session.execution);
        let mut rows = vec![SessionRow {
            reference: session.id,
            depth: 0,
            label: format!("Session {}{model_selection}", session.id),
            kind: SessionRowKind::Session,
            candidate: None,
            snapshot: None,
            transient: false,
        }];
        let mut visited = BTreeSet::new();
        let mut tool_invocation_details = std::collections::BTreeMap::new();
        let roots = session
            .branches
            .iter()
            .filter(|branch| branch.parent.is_none())
            .map(|branch| branch.id)
            .collect::<Vec<_>>();
        for branch in roots {
            append_branch(
                session,
                branch,
                1,
                &mut visited,
                &mut rows,
                &mut tool_invocation_details,
                expanded,
            );
        }
        for branch in SessionViews::branches(session) {
            if !visited.contains(&branch.id) {
                append_branch(
                    session,
                    branch.id,
                    1,
                    &mut visited,
                    &mut rows,
                    &mut tool_invocation_details,
                    expanded,
                );
            }
        }
        Self {
            rows,
            execution: session.execution.clone(),
            tool_invocation_details,
        }
    }

    /// Returns the stable semantic reference at a cursor position.
    ///
    /// - `cursor`: Row index to resolve.
    ///
    /// The row reference, when the cursor is in range.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn reference_at(&self, cursor: usize) -> Option<EntityId> {
        self.rows.get(cursor).map(|row| row.reference)
    }

    /// Replaces the shared selection and refreshes the visible session header.
    ///
    /// - `execution`: Authoritative future-invocation selection.
    pub fn set_execution(&mut self, execution: scriberig_session::SessionExecutionSelection) {
        if let Some(row) = self
            .rows
            .iter_mut()
            .find(|row| row.kind == SessionRowKind::Session)
        {
            row.label = format!("Session {}{}", row.reference, execution_label(&execution));
        }
        self.execution = execution;
    }
}

/// Formats the model and thinking portion of the session header.
///
/// - `execution`: Shared future-invocation selection.
///
/// A compact label suffix for the session header.
/// # Returns
/// The operation result.
fn execution_label(execution: &scriberig_session::SessionExecutionSelection) -> String {
    match (
        execution.model_id.as_deref(),
        execution.thinking_level.as_deref(),
    ) {
        (Some(model), Some(thinking)) => format!(" · Model {model} · Thinking {thinking}"),
        (Some(model), None) => format!(" · Model {model}"),
        _ => String::new(),
    }
}

/// Appends one branch and its nested activity in stored creation order.
///
/// - `session`: Session graph to traverse.
/// - `branch_id`: Branch identity to append.
/// - `depth`: Current tree depth.
/// - `visited`: Branches already traversed.
/// - `rows`: Output rows being built.
/// - `tool_invocation_details`: Safe details indexed by invocation identity.
/// # Parameters
/// - `expanded`: Input for this operation.
fn append_branch(
    session: &Session,
    branch_id: EntityId,
    depth: usize,
    visited: &mut BTreeSet<EntityId>,
    rows: &mut Vec<SessionRow>,
    tool_invocation_details: &mut std::collections::BTreeMap<EntityId, String>,
    expanded: &BTreeSet<EntityId>,
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
    let compact = source.parent.is_some()
        && source.status == BranchStatus::Completed
        && !expanded.contains(&source.id);
    if !compact
        && !session_activity::append(source, depth, transient, rows, tool_invocation_details)
    {
        session_turns::append(source, depth, transient, rows, tool_invocation_details);
    }
    append_result(source, depth, rows);
    append_children(
        session,
        source,
        depth,
        visited,
        rows,
        tool_invocation_details,
        expanded,
    );
}

/// Appends durable result, reference, and candidate rows for one branch.
///
/// - `source`: Branch whose durable result is projected.
/// - `depth`: Branch row depth.
/// - `rows`: Output rows being built.
fn append_result(source: &scriberig_session::Branch, depth: usize, rows: &mut Vec<SessionRow>) {
    if let Some(result) = source.result.as_ref() {
        rows.push(SessionRow {
            reference: result.id,
            depth: depth + 1,
            label: source
                .result
                .as_ref()
                .and_then(|result| result.summary.as_deref())
                .map_or_else(|| "Durable branch result".to_owned(), str::to_owned),
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

/// Recursively appends child branches in their stored creation order.
///
/// - `session`: Session graph containing child branches.
/// - `source`: Parent branch whose children are traversed.
/// - `depth`: Parent branch row depth.
/// - `visited`: Branches already traversed.
/// - `rows`: Output rows being built.
/// - `tool_invocation_details`: Safe details indexed by invocation identity.
/// # Parameters
/// - `expanded`: Input for this operation.
fn append_children(
    session: &Session,
    source: &scriberig_session::Branch,
    depth: usize,
    visited: &mut BTreeSet<EntityId>,
    rows: &mut Vec<SessionRow>,
    tool_invocation_details: &mut std::collections::BTreeMap<EntityId, String>,
    expanded: &BTreeSet<EntityId>,
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
    for child in children {
        append_branch(
            session,
            child,
            depth + 1,
            visited,
            rows,
            tool_invocation_details,
            expanded,
        );
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
