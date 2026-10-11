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
    /// # Parameters
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
    /// # Returns
    /// The structured result.
    ///
    /// # Errors
    /// Returns a validation or formatting error when inputs are invalid.
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
    /// # Parameters
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
    /// # Returns
    /// The structured result.
    ///
    /// # Errors
    /// Returns a validation or formatting error when inputs are invalid.
    pub fn turns(branch: &Branch) -> Vec<TurnView> {
        let mut r = branch.turns.iter().map(Self::turn).collect::<Vec<_>>();
        r.sort_by_key(|t| t.ordinal);
        r
    }
    #[must_use]
    /// Executes `decisions` for structured session state.
    ///
    /// # Parameters
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
    /// # Returns
    /// The structured result.
    ///
    /// # Errors
    /// Returns a validation or formatting error when inputs are invalid.
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
