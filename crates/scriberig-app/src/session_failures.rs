//! Retains terminal failures for application-owned child sessions.

use scriberig_session::BranchResult;

use crate::{ApplicationError, ApplicationService, LocalAgentHandle, LocalChildAgentHandle};

impl<B> ApplicationService<B> {
    /// Abandons a root branch while retaining its compact failure result.
    ///
    /// - `self`: Application service.
    /// - `handle`: Pending root operation handle.
    /// - `failure`: Bounded failure description.
    ///
    /// Unit after branch and operation failure are recorded.
    ///
    /// Returns an application error when the branch or operation is unknown.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn fail_local_agent(
        &self,
        handle: &LocalAgentHandle,
        failure: impl Into<String>,
    ) -> Result<(), ApplicationError> {
        self.fail_local_agent_with_model_requests(handle, failure, Vec::new())
    }

    /// Abandons a root branch and retains safe requests captured before failure.
    ///
    /// - `self`: Application service.
    /// - `handle`: Pending root operation handle.
    /// - `failure`: Bounded failure description safe for durable storage.
    /// - `model_requests`: Safe provider-neutral requests captured before failure.
    ///
    /// Unit after branch, request history, and operation failure are recorded.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn fail_local_agent_with_model_requests(
        &self,
        handle: &LocalAgentHandle,
        failure: impl Into<String>,
        model_requests: Vec<scriberig_session::ModelRequestProjection>,
    ) -> Result<(), ApplicationError> {
        let failure = failure.into();
        let cancelled =
            self.operations.get(&handle.operation)?.status == crate::OperationStatus::Cancelled;
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
        self.persist_model_requests(branch, model_requests);
        branch.outcome = Some(failure.clone());
        let result_id = self.next_entity(0xB8);
        branch.result = Some(BranchResult {
            id: result_id,
            branch: branch.id,
            status: scriberig_session::BranchStatus::Abandoned,
            summary: Some(failure.clone()),
            result_kind: Some(if cancelled {
                "agent-cancelled".to_owned()
            } else {
                "agent-failure".to_owned()
            }),
            candidate_context: None,
            references: Vec::new(),
            result_handles: Vec::new(),
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        });
        branch
            .transition(scriberig_session::BranchStatus::Abandoned)
            .map_err(|error| ApplicationError::Internal {
                message: error.to_string(),
            })?;
        self.save_session_state(session)?;
        if !cancelled {
            self.operations.fail(
                &handle.operation,
                ApplicationError::Internal { message: failure },
            )?;
        }
        Ok(())
    }

    /// Abandons a child branch while retaining its compact failure result.
    ///
    /// - `self`: Application service.
    /// - `handle`: Pending child operation handle.
    /// - `failure`: Bounded failure description.
    ///
    /// Unit after branch and operation failure are recorded.
    ///
    /// Returns an application error when the child or operation is unknown.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn fail_local_child_agent(
        &self,
        handle: &LocalChildAgentHandle,
        failure: impl Into<String>,
    ) -> Result<(), ApplicationError> {
        self.fail_local_child_agent_with_model_requests(handle, failure, Vec::new())
    }

    /// Abandons a child branch with its safely captured model requests.
    ///
    /// - `self`: Application service.
    /// - `handle`: Pending child operation handle.
    /// - `failure`: Bounded failure description safe for durable storage.
    /// - `model_requests`: Safe provider-neutral requests captured before failure.
    ///
    /// Unit after branch, request history, and operation failure are recorded.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn fail_local_child_agent_with_model_requests(
        &self,
        handle: &LocalChildAgentHandle,
        failure: impl Into<String>,
        model_requests: Vec<scriberig_session::ModelRequestProjection>,
    ) -> Result<(), ApplicationError> {
        let failure = failure.into();
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
        self.persist_model_requests(branch, model_requests);
        branch.outcome = Some(failure.clone());
        let result_id = self.next_entity(0xB7);
        branch.result = Some(BranchResult {
            id: result_id,
            branch: branch.id,
            status: scriberig_session::BranchStatus::Abandoned,
            summary: Some(failure.clone()),
            result_kind: Some("agent-child-failure".to_owned()),
            candidate_context: None,
            references: Vec::new(),
            result_handles: Vec::new(),
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        });
        branch
            .transition(scriberig_session::BranchStatus::Abandoned)
            .map_err(|error| ApplicationError::Internal {
                message: error.to_string(),
            })?;
        self.save_session_state(session)?;
        self.operations.fail(
            &handle.operation,
            ApplicationError::Internal { message: failure },
        )?;
        Ok(())
    }
}
