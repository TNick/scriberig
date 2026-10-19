//! Defines deterministic progressive session views.
use crate::{
    branch::{Branch, BranchStatus},
    turn::Turn,
    types::{Decision, Session},
};
use scriberig_core::EntityId;
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct BranchDescriptor {
    pub id: EntityId,
    pub title: String,
    pub status: BranchStatus,
    pub turn_count: usize,
    pub touched: Vec<EntityId>,
    pub outcome: Option<String>,
    pub parent: Option<EntityId>,
    pub child_count: usize,
    pub activity_count: usize,
    pub result: Option<EntityId>,
}

/// Provides a compact result for a child branch without loading its activity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildBranchSummary {
    /// Child branch identity.
    pub branch: EntityId,
    /// Child branch title.
    pub title: String,
    /// Child purpose.
    pub purpose: Option<String>,
    /// Terminal lifecycle status.
    pub status: BranchStatus,
    /// Compact outcome summary.
    pub outcome: Option<String>,
    /// Semantic result handles.
    pub result_handles: Vec<EntityId>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct TurnView {
    pub id: EntityId,
    pub ordinal: u64,
    pub has_human_input: bool,
    pub has_agent_output: bool,
    pub tool_invocation: Option<EntityId>,
    pub tool_result: Option<EntityId>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct DecisionEntry {
    pub id: EntityId,
    pub branch: EntityId,
    pub text: String,
}
/// Session-domain structured value.
pub struct SessionViews;
impl SessionViews {
    #[must_use]
    /// Executes `branches` for structured session state.
    ///
    /// - `self`: Session value or formatter.
    /// - `id`: Stable entity identifier when applicable.
    /// - `title`: Branch title when applicable.
    /// - `next`: Requested lifecycle state when applicable.
    /// - `turn`: Turn to append when applicable.
    /// - `entity`: Cross-domain entity to reference when applicable.
    /// - `session`: Session snapshot when applicable.
    /// - `branch`: Branch snapshot when applicable.
    /// - `decisions`: Decision entries when applicable.
    /// - `f`: Formatting destination when applicable.
    ///
    /// The structured result.
    ///
    /// Returns a validation or formatting error when inputs are invalid.
    /// # Returns
    /// The operation result.
    pub fn branches(session: &Session) -> Vec<BranchDescriptor> {
        let mut r = session
            .branches
            .iter()
            .map(Self::branch)
            .collect::<Vec<_>>();
        r.sort_by_key(|b| b.id);
        r
    }
    #[must_use]
    /// Executes `turns` for structured session state.
    ///
    /// - `self`: Session value or formatter.
    /// - `id`: Stable entity identifier when applicable.
    /// - `title`: Branch title when applicable.
    /// - `next`: Requested lifecycle state when applicable.
    /// - `turn`: Turn to append when applicable.
    /// - `entity`: Cross-domain entity to reference when applicable.
    /// - `session`: Session snapshot when applicable.
    /// - `branch`: Branch snapshot when applicable.
    /// - `decisions`: Decision entries when applicable.
    /// - `f`: Formatting destination when applicable.
    ///
    /// The structured result.
    ///
    /// Returns a validation or formatting error when inputs are invalid.
    /// # Returns
    /// The operation result.
    pub fn turns(branch: &Branch) -> Vec<TurnView> {
        let mut r = branch.turns.iter().map(Self::turn).collect::<Vec<_>>();
        r.sort_by_key(|t| t.ordinal);
        r
    }

    /// Returns compact child results from the parent session graph.
    ///
    /// - `session`: Session graph containing the branches.
    /// - `parent`: Parent branch whose children are summarized.
    ///
    /// Compact summaries sorted by branch identity.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn child_summaries(session: &Session, parent: EntityId) -> Vec<ChildBranchSummary> {
        let mut summaries = session
            .branches
            .iter()
            .filter(|branch| branch.parent == Some(parent))
            .map(|branch| ChildBranchSummary {
                branch: branch.id,
                title: branch.title.clone(),
                purpose: branch.purpose.clone(),
                status: branch.status,
                outcome: branch.outcome.clone(),
                result_handles: branch
                    .result
                    .as_ref()
                    .map(|result| result.references.clone())
                    .unwrap_or_default(),
            })
            .collect::<Vec<_>>();
        summaries.sort_by_key(|summary| summary.branch);
        summaries
    }
    #[must_use]
    /// Executes `decisions` for structured session state.
    ///
    /// - `self`: Session value or formatter.
    /// - `id`: Stable entity identifier when applicable.
    /// - `title`: Branch title when applicable.
    /// - `next`: Requested lifecycle state when applicable.
    /// - `turn`: Turn to append when applicable.
    /// - `entity`: Cross-domain entity to reference when applicable.
    /// - `session`: Session snapshot when applicable.
    /// - `branch`: Branch snapshot when applicable.
    /// - `decisions`: Decision entries when applicable.
    /// - `f`: Formatting destination when applicable.
    ///
    /// The structured result.
    ///
    /// Returns a validation or formatting error when inputs are invalid.
    /// # Returns
    /// The operation result.
    pub fn decisions(decisions: &[Decision]) -> Vec<DecisionEntry> {
        let mut r = decisions
            .iter()
            .map(|d| DecisionEntry {
                id: d.id,
                branch: d.branch,
                text: d.text.clone(),
            })
            .collect::<Vec<_>>();
        r.sort_by_key(|d| d.id);
        r
    }
    /// Builds compact branch metadata.\n    ///\n    /// # Parameters\n    /// - `b`: Branch snapshot.\n    ///\n    /// # Returns\n    /// Compact branch descriptor.
    fn branch(b: &Branch) -> BranchDescriptor {
        BranchDescriptor {
            id: b.id,
            title: b.title.clone(),
            status: b.status,
            turn_count: b.turns.len(),
            touched: b.touched.clone(),
            outcome: b.outcome.clone(),
            parent: b.parent,
            child_count: b.children.len(),
            activity_count: b.activity.len(),
            result: b.result.as_ref().map(|result| result.id),
        }
    }
    /// Builds one turn view.\n    ///\n    /// # Parameters\n    /// - `t`: Turn snapshot.\n    ///\n    /// # Returns\n    /// Compact turn view.
    fn turn(t: &Turn) -> TurnView {
        TurnView {
            id: t.id,
            ordinal: t.ordinal,
            has_human_input: t.human.is_some(),
            has_agent_output: t.agent.is_some(),
            tool_invocation: t.tool_invocation,
            tool_result: t.tool_result,
        }
    }
}
