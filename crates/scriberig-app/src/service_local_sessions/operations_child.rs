//! Implements child agent session operations.

use std::sync::atomic::Ordering;

use super::super::persist_execution_evidence;
use crate::{ApplicationError, ApplicationService};
use scriberig_core::EntityId;
use scriberig_session::{BranchResult, Turn};
impl<B> ApplicationService<B> {
    /// Creates a child branch and leaves its shared operation running.
    ///
    /// # Parameters
    /// - `self`: Application service owning local session state.
    /// - `session_id`: Session containing the parent branch.
    /// - `parent_branch`: Existing parent branch identifier.
    /// - `title`: Human-readable child title.
    /// - `purpose`: Focused child purpose.
    /// - `prompt`: Child model instruction.
    ///
    /// # Returns
    /// A handle used to complete the child operation.
    ///
    /// # Errors
    /// Returns an application error when the session graph is unavailable.
    pub fn start_local_child_agent(
        &self,
        session_id: EntityId,
        parent_branch: EntityId,
        title: impl Into<String>,
        purpose: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Result<crate::LocalChildAgentHandle, ApplicationError> {
        let title = title.into();
        let purpose = purpose.into();
        let prompt = prompt.into();
        let child_branch = self.next_entity(0xB4);
        let mut session =
            self.load_session_state(session_id)?
                .ok_or_else(|| ApplicationError::NotFound {
                    subject: session_id.to_string(),
                })?;
        session
            .add_child_branch(child_branch, parent_branch, title, purpose.clone())
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "agent_child_start".to_owned(),
                message: error.to_string(),
            })?;
        if let Some(parent) = session
            .branches
            .iter_mut()
            .find(|branch| branch.id == parent_branch)
        {
            parent
                .activity
                .push(scriberig_session::ExecutionActivity::ChildBranch(
                    child_branch,
                ));
        }
        let turn_id = self.next_entity(0xB9);
        if let Some(child) = session
            .branches
            .iter_mut()
            .find(|branch| branch.id == child_branch)
        {
            child.push_turn(Turn {
                id: turn_id,
                ordinal: 0,
                human: Some(scriberig_session::HumanInput {
                    text: prompt.clone(),
                }),
                agent: None,
                tool_invocation: None,
                tool_result: None,
            });
            child
                .activity
                .push(scriberig_session::ExecutionActivity::HumanInput(turn_id));
        }
        self.save_session_state(session)?;
        let sequence = self.next_agent_handle.fetch_add(1, Ordering::Relaxed);
        let operation = scriberig_core::OperationId::new(format!("agent-child-{sequence}"))
            .map_err(|error| ApplicationError::Internal {
                message: error.to_string(),
            })?;
        self.operations
            .start(operation.clone(), crate::OperationKind::Branch)?;
        self.operations
            .transition(&operation, crate::OperationStatus::Running)?;
        Ok(crate::LocalChildAgentHandle {
            operation,
            session: session_id,
            parent_branch,
            child_branch,
            purpose,
            prompt,
        })
    }

    /// Completes a child operation and records its durable branch result.
    ///
    /// # Parameters
    /// - `self`: Application service owning local session state.
    /// - `handle`: Pending child operation handle.
    /// - `final_output`: Complete externally meaningful child output.
    ///
    /// # Returns
    /// The structured application agent result for the child.
    ///
    /// # Errors
    /// Returns an application error when the child branch is unavailable.
    pub fn complete_local_child_agent(
        &self,
        handle: &crate::LocalChildAgentHandle,
        final_output: impl Into<String>,
    ) -> Result<crate::AgentResult, ApplicationError> {
        self.complete_local_child_agent_with_handles(handle, final_output, Vec::new())
    }

    /// Completes a child branch with compact semantic result references.
    ///
    /// # Parameters
    /// - `self`: Application service owning local session state.
    /// - `handle`: Pending child operation handle.
    /// - `final_output`: Complete externally meaningful child output.
    /// - `result_handles`: Stable semantic references returned by the child.
    ///
    /// # Returns
    /// The structured application agent result for the child.
    ///
    /// # Errors
    /// Returns an application error when the child branch is unavailable.
    pub fn complete_local_child_agent_with_handles(
        &self,
        handle: &crate::LocalChildAgentHandle,
        final_output: impl Into<String>,
        result_handles: Vec<String>,
    ) -> Result<crate::AgentResult, ApplicationError> {
        self.complete_local_child_agent_with_evidence(
            handle,
            final_output,
            result_handles,
            Vec::new(),
            Vec::new(),
        )
    }

    /// Completes a child branch and persists its executed tool evidence.
    ///
    /// # Parameters
    /// - `self`: Application service owning local session state.
    /// - `handle`: Pending child operation handle.
    /// - `final_output`: Complete externally meaningful child output.
    /// - `result_handles`: Stable semantic references returned by the child.
    /// - `tool_evidence`: Exact tool arguments and bounded structured results.
    /// - `model_requests`: Safe normalized requests captured by the runtime.
    ///
    /// # Returns
    /// The structured application agent result for the child.
    ///
    /// # Errors
    /// Returns an application error when the child branch is unavailable.
    pub fn complete_local_child_agent_with_evidence(
        &self,
        handle: &crate::LocalChildAgentHandle,
        final_output: impl Into<String>,
        result_handles: Vec<String>,
        tool_evidence: Vec<crate::ToolExecutionEvidence>,
        model_requests: Vec<scriberig_session::ModelRequestProjection>,
    ) -> Result<crate::AgentResult, ApplicationError> {
        let final_output = final_output.into();
        let mut session =
            self.load_session_state(handle.session)?
                .ok_or_else(|| ApplicationError::NotFound {
                    subject: handle.session.to_string(),
                })?;
        let branch = session
            .branches
            .iter_mut()
            .find(|branch| branch.id == handle.child_branch)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: handle.child_branch.to_string(),
            })?;
        persist_execution_evidence(self, branch, tool_evidence, model_requests);
        let output_turn = if let Some(turn) = branch.turns.iter_mut().rev().find(|turn| {
            turn.agent.is_none()
                && turn
                    .human
                    .as_ref()
                    .is_some_and(|human| human.text == handle.prompt)
        }) {
            turn.agent = Some(scriberig_session::AgentOutput {
                text: final_output.clone(),
            });
            turn.id
        } else {
            let turn_id = self.next_entity(0xB5);
            branch.push_turn(Turn {
                id: turn_id,
                ordinal: 0,
                human: Some(scriberig_session::HumanInput {
                    text: handle.prompt.clone(),
                }),
                agent: Some(scriberig_session::AgentOutput {
                    text: final_output.clone(),
                }),
                tool_invocation: None,
                tool_result: None,
            });
            branch
                .activity
                .push(scriberig_session::ExecutionActivity::HumanInput(turn_id));
            turn_id
        };
        branch
            .activity
            .push(scriberig_session::ExecutionActivity::AgentOutput(
                output_turn,
            ));
        let branch_result = self.next_entity(0xB6);
        branch.result = Some(BranchResult {
            id: branch_result,
            branch: branch.id,
            status: scriberig_session::BranchStatus::Completed,
            summary: Some(final_output.clone()),
            result_kind: Some("agent-child".to_owned()),
            references: vec![branch_result],
            result_handles: result_handles.clone(),
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        });
        branch
            .activity
            .push(scriberig_session::ExecutionActivity::BranchResult(
                branch_result,
            ));
        branch
            .transition(scriberig_session::BranchStatus::Completed)
            .map_err(|error| ApplicationError::Internal {
                message: error.to_string(),
            })?;
        self.save_session_state(session)?;
        let result = crate::AgentResult {
            session: handle.session,
            root_branch: handle.child_branch,
            final_output,
            branch_result,
            produced_changes: Vec::new(),
            validation_results: Vec::new(),
            unresolved_questions: Vec::new(),
            candidate_changes: Vec::new(),
        };
        self.operations
            .publish_agent_output(&handle.operation, &result.final_output)?;
        self.operations.complete_agent(
            &handle.operation,
            std::iter::once(branch_result.to_string())
                .chain(result_handles)
                .collect(),
            result.clone(),
        )?;
        Ok(result)
    }
}
