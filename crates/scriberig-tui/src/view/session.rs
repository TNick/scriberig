//! Projects provider-neutral sessions into stable terminal tree rows.

use std::collections::BTreeSet;

use scriberig_core::EntityId;
use scriberig_session::Session;

#[path = "session_activity.rs"]
mod session_activity;
#[path = "session_projection.rs"]
mod session_projection;
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

/// Identifies a session row independently of its flattened projection index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionRowIdentity {
    /// Stable semantic object represented by the row.
    pub reference: EntityId,
    /// Public session object kind.
    pub kind: SessionRowKind,
}

impl SessionRow {
    /// Returns this row's stable identity for selection across projection refreshes.
    ///
    /// # Returns
    /// The kind and semantic reference that identify this row.
    #[must_use]
    pub fn identity(&self) -> SessionRowIdentity {
        SessionRowIdentity {
            reference: self.reference,
            kind: self.kind,
        }
    }
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
        let tool_invocation_details =
            session_projection::append_branches(session, expanded, &mut rows);
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

    /// Finds the current row index for a stable session row identity.
    ///
    /// - `identity`: Semantic row identity from an earlier projection.
    ///
    /// The row's current flattened index when it remains present.
    /// # Returns
    /// The current row index when the stable identity remains in the view.
    #[must_use]
    pub fn index_of(&self, identity: SessionRowIdentity) -> Option<usize> {
        self.rows.iter().position(|row| row.identity() == identity)
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

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
