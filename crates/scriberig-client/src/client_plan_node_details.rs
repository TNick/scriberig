//! Provides a focused Rust client method for node gate and evidence pages.

use super::Client;
use crate::error::ClientError;
use scriberig_protocol::{
    ProtocolTransport, Request, Response, SessionPlanNodeDetailsPage, SessionPlanNodeDetailsRequest,
};

impl<T: ProtocolTransport> Client<T> {
    /// Reads one bounded page of a node's gates and evidence.
    ///
    /// # Parameters
    /// - `request`: Session, plan, node, snapshot, and page range.
    ///
    /// # Returns
    /// Gate definitions and associated evidence for the requested page.
    ///
    /// # Errors
    /// Returns a structured server, transport, or response-mismatch error.
    pub fn plan_node_details(
        &self,
        request: SessionPlanNodeDetailsRequest,
    ) -> Result<SessionPlanNodeDetailsPage, ClientError> {
        match self.send(Request::PlanNodeDetails(request))?.response {
            Response::PlanNodeDetails(page) => Ok(page),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
