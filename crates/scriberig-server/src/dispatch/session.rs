//! Dispatches durable session and model-request detail reads.

use super::{Dispatcher, helpers};
use crate::server::ApplicationService;
use scriberig_protocol::{Response, SessionPlanAttemptsRequest, SessionReadRequest};

impl<S: ApplicationService> Dispatcher<S> {
    /// Reads one bounded page of retained attempts for a plan node.
    ///
    /// # Parameters
    /// - `self`: Protocol dispatcher.
    /// - `request`: Session, plan, node, and paging identities.
    /// - `correlation_id`: Request correlation identity.
    ///
    /// # Returns
    /// One safe page of attempt metadata or a structured protocol error.
    pub(super) fn plan_attempts_read(
        &self,
        request: &SessionPlanAttemptsRequest,
        correlation_id: &str,
    ) -> Response {
        self.service
            .read_plan_attempts(&super::command_context("session_branch_tree"), request)
            .map_or_else(
                |error| Response::Error(helpers::map_error(&error, correlation_id)),
                Response::PlanAttempts,
            )
    }

    /// Reads one durable session through the application boundary.
    ///
    /// - `self`: Protocol dispatcher.
    /// - `request`: Typed session read request.
    /// - `correlation_id`: Request correlation identity.
    ///
    /// Typed session response or protocol error.
    /// # Returns
    /// The operation result.
    pub(super) fn session_read(
        &self,
        request: &SessionReadRequest,
        correlation_id: &str,
    ) -> Response {
        self.service
            .read_session(
                &super::command_context("session_branch_tree"),
                &request.session.id,
            )
            .map_or_else(
                |error| Response::Error(helpers::map_error(&error, correlation_id)),
                |response| {
                    Response::Session(request.branch.as_ref().map_or(response.clone(), |branch| {
                        helpers::select_branch_subtree(response, &branch.id)
                    }))
                },
            )
    }
}
