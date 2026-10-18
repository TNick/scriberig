//! Starts and scopes local application agent continuations.

use std::sync::atomic::Ordering;

use super::super::ApplicationService;
use crate::ApplicationError;
use scriberig_session::{Branch, BranchStatus, Session, SessionExecutionSelection, Turn};

impl<B> ApplicationService<B> {
    /// Creates a local session root and leaves its shared operation running.
    ///
    /// # Parameters
    /// - `self`: Application service owning local session state.
    /// - `request`: Session, optional branch, and human instruction.
    ///
    /// # Returns
    /// A handle used to complete the same operation after runtime execution.
    ///
    /// # Errors
    /// Returns an application error when session or operation state is invalid.
    pub fn start_local_agent(
        &self,
        request: &crate::AgentCommandRequest,
    ) -> Result<crate::LocalAgentHandle, ApplicationError> {
        let mut session = self
            .load_session_state(request.session)?
            .unwrap_or(Session {
                id: request.session,
                branches: Vec::new(),
                goals: Vec::new(),
                tasks: Vec::new(),
                execution: SessionExecutionSelection::default(),
            });
        let branch_id = if let Some(parent) = request.branch {
            let existing = session
                .branches
                .iter()
                .find(|branch| branch.id == parent)
                .ok_or_else(|| ApplicationError::NotFound {
                    subject: parent.to_string(),
                })?;
            if matches!(
                existing.status,
                BranchStatus::Completed | BranchStatus::Abandoned
            ) {
                let continuation = self.next_entity(0xB1);
                session
                    .add_child_branch(
                        continuation,
                        parent,
                        "agent continuation",
                        "continue completed branch",
                    )
                    .map_err(|error| ApplicationError::InvalidRequest {
                        operation: "agent_continue".to_owned(),
                        message: error.to_string(),
                    })?;
                continuation
            } else {
                parent
            }
        } else {
            self.next_entity(0xB1)
        };
        if !session.branches.iter().any(|branch| branch.id == branch_id) {
            let branch = Branch::new(branch_id, "agent root").map_err(|error| {
                ApplicationError::InvalidRequest {
                    operation: "agent_start".to_owned(),
                    message: error.to_string(),
                }
            })?;
            session.branches.push(branch);
        }
        if let Some(branch) = session
            .branches
            .iter_mut()
            .find(|branch| branch.id == branch_id)
        {
            let turn_id = self.next_entity(0xBA);
            branch.push_turn(Turn {
                id: turn_id,
                ordinal: 0,
                human: Some(scriberig_session::HumanInput {
                    text: request.prompt.clone(),
                }),
                agent: None,
                tool_invocation: None,
                tool_result: None,
            });
            branch
                .activity
                .push(scriberig_session::ExecutionActivity::HumanInput(turn_id));
        }
        self.save_session_state(session)?;
        let sequence = self.next_agent_handle.fetch_add(1, Ordering::Relaxed);
        let operation = scriberig_core::OperationId::new(format!("agent-local-{sequence}"))
            .map_err(|error| ApplicationError::Internal {
                message: error.to_string(),
            })?;
        self.operations
            .start(operation.clone(), crate::OperationKind::Agent)?;
        self.operations
            .transition(&operation, crate::OperationStatus::Running)?;
        Ok(crate::LocalAgentHandle {
            operation,
            session: request.session,
            root_branch: branch_id,
            prompt: request.prompt.clone(),
        })
    }
}
