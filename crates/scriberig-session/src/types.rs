//! Defines session-level entities and cross-domain handles.
use crate::branch::Branch;
use crate::execution::SessionExecutionSelection;
use scriberig_core::{EntityId, Provenance};
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Session {
    pub id: EntityId,
    pub branches: Vec<Branch>,
    pub goals: Vec<Goal>,
    pub tasks: Vec<Task>,
    /// Durable selection used as the default for future model invocations.
    #[serde(default)]
    pub execution: SessionExecutionSelection,
}

impl Session {
    /// Inserts a child branch and updates the parent adjacency list.
    ///
    /// # Parameters
    /// - `self`: Session graph being updated.
    /// - `id`: Stable child branch identifier.
    /// - `parent`: Existing parent branch identifier.
    /// - `title`: Human-readable child title.
    /// - `purpose`: Focused work assigned to the child.
    ///
    /// # Returns
    /// Unit after the child is linked into the session graph.
    ///
    /// # Errors
    /// Returns `MissingParent` when the parent is absent and `DuplicateBranch`
    /// when the child identifier is already present.
    pub fn add_child_branch(
        &mut self,
        id: EntityId,
        parent: EntityId,
        title: impl Into<String>,
        purpose: impl Into<String>,
    ) -> Result<(), crate::SessionError> {
        if self.branches.iter().any(|branch| branch.id == id) {
            return Err(crate::SessionError::DuplicateBranch);
        }
        let parent_branch = self
            .branches
            .iter_mut()
            .find(|branch| branch.id == parent)
            .ok_or(crate::SessionError::MissingParent)?;
        let child = Branch::new_child(id, parent, title, purpose)?;
        parent_branch.children.push(id);
        parent_branch.children.sort();
        self.branches.push(child);
        self.branches.sort_by_key(|branch| branch.id);
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Goal {
    pub id: EntityId,
    pub owner: EntityId,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Task {
    pub id: EntityId,
    pub owner: EntityId,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Observation {
    pub id: EntityId,
    pub text: String,
    pub provenance: Provenance,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Question {
    pub id: EntityId,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Decision {
    pub id: EntityId,
    pub branch: EntityId,
    pub text: String,
    pub provenance: Provenance,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct BranchResult {
    pub id: EntityId,
    pub branch: EntityId,
    pub status: crate::branch::BranchStatus,
    pub summary: Option<String>,
    pub result_kind: Option<String>,
    pub references: Vec<EntityId>,
    /// Stable non-entity handles produced by the branch.
    #[serde(default)]
    pub result_handles: Vec<String>,
    pub touched: Vec<EntityId>,
    pub produced_changes: Vec<EntityId>,
    pub validations: Vec<EntityId>,
    pub unresolved_questions: Vec<EntityId>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct Summary {
    pub id: EntityId,
    pub text: String,
    pub provenance: Provenance,
}
