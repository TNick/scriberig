//! Dispatches bounded plan-node gate and evidence detail reads.

use std::str::FromStr;

use super::{ApplicationService, Dispatcher, Response, command_context, helpers};
use scriberig_app::{ApplicationError, PlanNodeDetailsRequest};
use scriberig_core::{EntityId, SnapshotId};

impl<S: ApplicationService> Dispatcher<S> {
    /// Reads exact-snapshot node details and projects the bounded page.
    ///
    /// # Parameters
    /// - `request`: Typed protocol query with string identities.
    /// - `correlation_id`: Request correlation identity.
    ///
    /// # Returns
    /// Bounded detail page or structured protocol error.
    pub(super) fn plan_node_details(
        &self,
        request: &scriberig_protocol::SessionPlanNodeDetailsRequest,
        correlation_id: &str,
    ) -> Response {
        let app_request = (|| {
            Ok(PlanNodeDetailsRequest {
                session: parse_entity(&request.session.id, "session")?,
                plan: parse_entity(&request.plan, "plan")?,
                node: parse_entity(&request.node, "node")?,
                snapshot: SnapshotId::from_str(&request.snapshot)
                    .map_err(|error| invalid_identity("snapshot", &error.to_string()))?,
                offset: request.offset,
                limit: request.limit,
            })
        })();
        let details = match app_request.and_then(|app_request| {
            self.service
                .read_plan_node_details(&command_context("session_branch_tree"), &app_request)
        }) {
            Ok(details) => details,
            Err(error) => return Response::Error(helpers::map_error(&error, correlation_id)),
        };
        Response::PlanNodeDetails(crate::server::server_helpers::project_session_plan_details(
            request.offset,
            &details,
        ))
    }
}

/// Parses one stable entity identity from a protocol string.
///
/// # Parameters
/// - `value`: String-encoded entity identity.
/// - `field`: Request field name for diagnostics.
///
/// # Returns
/// The parsed identity.
///
/// # Errors
/// Returns an invalid-request error for malformed identities.
fn parse_entity(value: &str, field: &str) -> Result<EntityId, ApplicationError> {
    EntityId::from_str(value).map_err(|error| invalid_identity(field, &error.to_string()))
}

/// Creates a structured invalid-identity application error.
///
/// # Parameters
/// - `field`: Request field name.
/// - `message`: Safe parser diagnostic.
///
/// # Returns
/// The structured invalid-request error.
fn invalid_identity(field: &str, message: &str) -> ApplicationError {
    ApplicationError::InvalidRequest {
        operation: "plan node details read".to_owned(),
        message: format!("invalid {field} identity: {message}"),
    }
}
