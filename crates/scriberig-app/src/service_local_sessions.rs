//! Provides local session lifecycle operations for the application service.

use std::sync::atomic::Ordering;

use super::ApplicationService;
use crate::ApplicationError;
use scriberig_core::EntityId;
use scriberig_session::{Branch, BranchResult, Session, Turn};

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
        let branch_id = request.branch.unwrap_or_else(|| self.next_entity(0xB1));
        let mut session = self
            .load_session_state(request.session)?
            .unwrap_or(Session {
                id: request.session,
                branches: Vec::new(),
                goals: Vec::new(),
                tasks: Vec::new(),
            });
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
            branch.push_turn(Turn {
                id: self.next_entity(0xBA),
                ordinal: 0,
                human: Some(scriberig_session::HumanInput {
                    text: request.prompt.clone(),
                }),
                agent: None,
                tool_invocation: None,
                tool_result: None,
            });
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
        self.save_session_state(session)?;
        let sequence = self.next_agent_handle.fetch_add(1, Ordering::Relaxed);
        let operation = scriberig_core::OperationId::new(format!("agent-child-{sequence}"))
            .map_err(|error| ApplicationError::Internal {
                message: error.to_string(),
            })?;
        self.operations
            .start(operation.clone(), crate::OperationKind::Agent)?;
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
        branch.push_turn(Turn {
            id: self.next_entity(0xB5),
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
        let branch_result = self.next_entity(0xB6);
        branch.result = Some(BranchResult {
            id: branch_result,
            branch: branch.id,
            status: scriberig_session::BranchStatus::Completed,
            summary: Some(final_output.clone()),
            result_kind: Some("agent-child".to_owned()),
            references: vec![branch_result],
            result_handles: Vec::new(),
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        });
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
            vec![branch_result.to_string()],
            result.clone(),
        )?;
        Ok(result)
    }

    /// Commits final runtime output into a pending local agent operation.
    ///
    /// # Parameters
    /// - `self`: Application service owning local session state.
    /// - `handle`: Pending operation and branch handle.
    /// - `final_output`: Complete externally meaningful agent output.
    ///
    /// # Returns
    /// The structured application agent result stored on the operation.
    ///
    /// # Errors
    /// Returns an application error when the branch or operation is unavailable.
    pub fn complete_local_agent(
        &self,
        handle: &crate::LocalAgentHandle,
        final_output: impl Into<String>,
    ) -> Result<crate::AgentResult, ApplicationError> {
        self.complete_local_agent_with_handles(handle, final_output, Vec::new())
    }

    /// Commits final output and stable result handles into a local agent branch.
    ///
    /// # Parameters
    /// - `self`: Application service owning local session state.
    /// - `handle`: Pending operation and branch handle.
    /// - `final_output`: Complete externally meaningful agent output.
    /// - `result_handles`: Stable project, snapshot, manifest, or candidate handles.
    ///
    /// # Returns
    /// The structured application agent result stored on the operation.
    ///
    /// # Errors
    /// Returns an application error when the branch or operation is unavailable.
    pub fn complete_local_agent_with_handles(
        &self,
        handle: &crate::LocalAgentHandle,
        final_output: impl Into<String>,
        result_handles: Vec<String>,
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
            .find(|branch| branch.id == handle.root_branch)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: handle.root_branch.to_string(),
            })?;
        if let Some(turn) = branch.turns.iter_mut().rev().find(|turn| {
            turn.agent.is_none()
                && turn
                    .human
                    .as_ref()
                    .is_some_and(|human| human.text == handle.prompt)
        }) {
            turn.agent = Some(scriberig_session::AgentOutput {
                text: final_output.clone(),
            });
        } else {
            branch.push_turn(Turn {
                id: self.next_entity(0xB2),
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
        }
        let branch_result = self.next_entity(0xB3);
        branch.result = Some(BranchResult {
            id: branch_result,
            branch: branch.id,
            status: scriberig_session::BranchStatus::Completed,
            summary: Some(final_output.clone()),
            result_kind: Some("agent".to_owned()),
            references: Vec::new(),
            result_handles: result_handles.clone(),
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        });
        branch
            .transition(scriberig_session::BranchStatus::Completed)
            .map_err(|error| ApplicationError::Internal {
                message: error.to_string(),
            })?;
        self.save_session_state(session)?;
        let result = crate::AgentResult {
            session: handle.session,
            root_branch: handle.root_branch,
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

    /// Accepts an agent request in the provider-neutral local session store.
    ///
    /// # Parameters
    /// - `super`: Parent application service module.
    /// - `self`: Application service owning session state.
    /// - `request`: Session, branch, and prompt supplied by the caller.
    ///
    /// # Returns
    /// A lifecycle operation identifier accepted for execution.
    ///
    /// # Errors
    /// Returns an application error when session state cannot be updated.
    pub(super) fn accept_local_agent(
        &self,
        request: &crate::AgentCommandRequest,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        let handle = self.start_local_agent(request)?;
        self.complete_local_agent(&handle, request.prompt.clone())?;
        Ok(handle.operation)
    }
}
