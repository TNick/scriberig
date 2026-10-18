//! Dispatches durable session and model-request detail reads.

use super::{Dispatcher, helpers};
use crate::server::ApplicationService;
use scriberig_protocol::{Response, SessionReadRequest};

impl<S: ApplicationService> Dispatcher<S> {
    /// Reads one durable session through the application boundary.
    ///
    /// # Parameters
    /// - `self`: Protocol dispatcher.
    /// - `request`: Typed session read request.
    /// - `correlation_id`: Request correlation identity.
    ///
    /// # Returns
    /// Typed session response or protocol error.
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
