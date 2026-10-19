//! Projects durable session and plan state into the conversation home tree.
use std::collections::BTreeSet;

use scriberig_core::EntityId;
use scriberig_session::Session;

#[path = "conversation_structure.rs"]
mod structure;
use super::session::{SessionModel, SessionRowKind};
use structure::{append_plan, relocate_continuation, relocate_plan_worker_branches};

/// Stable source identity retained by one conversation row.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConversationSource {
    /// Session root descriptor.
    Session(EntityId),
    /// Session branch descriptor.
    Branch(EntityId),
    /// Durable branch activity record.
    Activity(EntityId),
    /// Durable branch result.
    BranchResult(EntityId),
    /// Durable plan descriptor.
    Plan(EntityId),
    /// Durable plan node.
    PlanNode(EntityId),
    /// Transient activity attached to a stable runtime branch.
    TransientActivity(EntityId),
}

/// Classifies a rendered conversation row without storing canonical state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversationRowKind {
    /// Session root descriptor.
    Session,
    /// Branch descriptor.
    Branch,
    /// Human or assistant conversation message.
    Message,
    /// Model, tool, validation, or runtime activity.
    Activity,
    /// Compact branch result.
    BranchResult,
    /// Plan descriptor.
    Plan,
    /// Structured plan work item.
    PlanNode,
}

/// One flattened conversation row with stable durable source identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversationRow {
    /// Semantic source retained across rerenders and layout changes.
    pub source: ConversationSource,
    /// Exact shared continuation target when this row is replyable.
    pub reply_anchor: Option<scriberig_session::ContinuationAnchor>,
    /// Derived tree depth used to paint ancestry rails.
    pub depth: usize,
    /// Safe user-facing label.
    pub label: String,
    /// Row presentation category.
    pub kind: ConversationRowKind,
}

/// Presentation projection built from canonical session and plan values.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConversationModel {
    /// Structurally ordered conversation and plan rows.
    pub rows: Vec<ConversationRow>,
    /// Plan and node currently selected by durable coordinator state.
    pub coordinator_cursor: Option<(EntityId, EntityId)>,
}

impl ConversationModel {
    /// Projects a durable session snapshot and all contained plans.
    ///
    /// The existing shared session projection owns activity rendering details;
    /// this product projection adds plan structure and retains stable references.
    ///
    /// - `session`: Authoritative session snapshot.
    ///
    /// Rows ordered by session structure and plan-node parent relations.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn from_session(session: &Session) -> Self {
        Self::from_session_with_expanded(session, &BTreeSet::new())
    }

    /// Projects a session with selected completed branches expanded.
    ///
    /// - `session`: Authoritative session snapshot.
    /// - `expanded`: Stable branch identities whose activity should be shown.
    ///
    /// Structurally ordered rows with local expansion applied.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn from_session_with_expanded(session: &Session, expanded: &BTreeSet<EntityId>) -> Self {
        let shared = SessionModel::from_session_with_expanded(session, expanded);
        let mut rows = conversation_rows(session, &shared.rows);
        for branch in &session.branches {
            if let Some(origin) = branch.continuation {
                relocate_continuation(origin.anchor, branch.id, &mut rows);
            }
        }
        for plan in &session.plans {
            append_plan(session, plan, &mut rows);
        }
        relocate_plan_worker_branches(session, &mut rows);
        let coordinator_cursor = session
            .plans
            .iter()
            .find_map(|plan| plan.coordinator_cursor.map(|node| (plan.id, node)));
        Self {
            rows,
            coordinator_cursor,
        }
    }

    /// Resolves a stable conversation source by its semantic identity.
    ///
    /// - `source`: Durable source identity to locate.
    ///
    /// The current row index when the source remains projected.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn index_of(&self, source: ConversationSource) -> Option<usize> {
        self.rows.iter().position(|row| row.source == source)
    }

    /// Inserts one transient status row below its owning branch subtree.
    ///
    /// - `branch`: Stable branch identity owning the observation.
    /// - `key`: Stable client presentation identity for the operation activity.
    /// - `label`: Bounded safe status label.
    pub fn insert_transient_activity(&mut self, branch: EntityId, key: u64, label: String) {
        let Some(branch_index) = self
            .rows
            .iter()
            .position(|row| row.source == ConversationSource::Branch(branch))
        else {
            return;
        };
        let depth = self.rows[branch_index].depth;
        let insertion = self.rows[branch_index + 1..]
            .iter()
            .position(|row| row.depth <= depth)
            .map_or(self.rows.len(), |offset| branch_index + 1 + offset);
        let mut identity = [0; 16];
        identity[..8].copy_from_slice(&key.to_le_bytes());
        identity[8..].copy_from_slice(&branch.as_bytes()[..8]);
        self.rows.insert(
            insertion,
            ConversationRow {
                source: ConversationSource::TransientActivity(EntityId::from_bytes(identity)),
                reply_anchor: None,
                depth: depth.saturating_add(1),
                label,
                kind: ConversationRowKind::Activity,
            },
        );
    }
}

/// Converts an existing session row to its typed conversation source.
///
/// - `session`: Owning session identity.
/// - `row`: Existing shared projection row.
///
/// A conversation row retaining the semantic source.
/// # Returns
/// The operation result.
fn conversation_row(session: EntityId, row: &super::session::SessionRow) -> ConversationRow {
    let (source, kind) = match row.kind {
        SessionRowKind::Session => (
            ConversationSource::Session(session),
            ConversationRowKind::Session,
        ),
        SessionRowKind::Branch => (
            ConversationSource::Branch(row.reference),
            ConversationRowKind::Branch,
        ),
        SessionRowKind::Result => (
            ConversationSource::BranchResult(row.reference),
            ConversationRowKind::BranchResult,
        ),
        SessionRowKind::Turn | SessionRowKind::ModelOutput => (
            ConversationSource::Activity(row.reference),
            ConversationRowKind::Message,
        ),
        _ => (
            ConversationSource::Activity(row.reference),
            ConversationRowKind::Activity,
        ),
    };
    ConversationRow {
        source,
        reply_anchor: None,
        depth: row.depth,
        label: if kind == ConversationRowKind::Session {
            "Conversation".to_owned()
        } else {
            row.label.clone()
        },
        kind,
    }
}

/// Converts session rows while retaining their current branch and exact anchors.
///
/// - `session`: Canonical session snapshot containing branch and turn sources.
/// - `source_rows`: Existing shared session projection rows.
///
/// Conversation rows with stable sources and only semantically valid anchors.
/// # Returns
/// The operation result.
fn conversation_rows(
    session: &Session,
    source_rows: &[super::session::SessionRow],
) -> Vec<ConversationRow> {
    let mut branch_path = Vec::<(EntityId, usize)>::new();
    source_rows
        .iter()
        .map(|row| {
            while branch_path
                .last()
                .is_some_and(|(_, depth)| *depth >= row.depth)
                && row.kind != SessionRowKind::Branch
            {
                branch_path.pop();
            }
            let current_branch = if row.kind == SessionRowKind::Branch {
                while branch_path
                    .last()
                    .is_some_and(|(_, depth)| *depth >= row.depth)
                {
                    branch_path.pop();
                }
                branch_path.push((row.reference, row.depth));
                Some(row.reference)
            } else {
                branch_path.last().map(|(branch, _)| *branch)
            };
            let mut projected = conversation_row(session.id, row);
            projected.reply_anchor = match row.kind {
                SessionRowKind::Branch => Some(scriberig_session::ContinuationAnchor::BranchHead {
                    branch: row.reference,
                }),
                SessionRowKind::Turn | SessionRowKind::ModelOutput => current_branch
                    .and_then(|branch| session.branches.iter().find(|item| item.id == branch))
                    .and_then(|branch| branch.turns.iter().find(|turn| turn.id == row.reference))
                    .and_then(|turn| {
                        if turn.agent.is_some() {
                            Some(scriberig_session::ContinuationAnchor::AssistantOutput {
                                branch: current_branch?,
                                turn: turn.id,
                            })
                        } else if turn.human.is_some() {
                            Some(scriberig_session::ContinuationAnchor::HumanInput {
                                branch: current_branch?,
                                turn: turn.id,
                            })
                        } else {
                            None
                        }
                    }),
                SessionRowKind::Result => current_branch
                    .and_then(|branch| session.branches.iter().find(|item| item.id == branch))
                    .and_then(|branch| branch.result.as_ref())
                    .filter(|result| result.id == row.reference)
                    .map(
                        |result| scriberig_session::ContinuationAnchor::BranchResult {
                            branch: current_branch.unwrap_or(session.id),
                            result: result.id,
                        },
                    ),
                _ => None,
            };
            projected
        })
        .collect()
}

#[cfg(test)]
#[path = "conversation_tests.rs"]
mod tests;
