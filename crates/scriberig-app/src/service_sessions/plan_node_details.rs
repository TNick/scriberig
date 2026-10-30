//! Reads bounded plan-node gate and evidence details through session storage.

use super::super::{ApplicationBackend, ApplicationService};
use super::authorize_session_read;
use crate::{ApplicationContext, ApplicationError, PlanNodeDetailsRequest, SessionRequest};

/// Reads one page of gate and evidence detail for a plan node.
///
/// # Parameters
/// - `service`: Application service with its configured semantic store.
/// - `context`: Authorized request context.
/// - `request`: Node, exact snapshot, and page range to inspect.
///
/// # Returns
/// The requested gates and associated evidence.
///
/// # Errors
/// Returns authorization, invalid-page, history, or persistence failures.
pub(super) fn read<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    request: &PlanNodeDetailsRequest,
) -> Result<scriberig_session::PlanSemanticNodeDetails, ApplicationError> {
    authorize_session_read(
        context,
        &SessionRequest {
            session: request.session,
        },
    )?;
    if request.limit == 0 || request.limit > 256 {
        return Err(ApplicationError::InvalidRequest {
            operation: "plan node details read".to_owned(),
            message: "limit must be between 1 and 256".to_owned(),
        });
    }
    let store =
        service
            .session_store
            .as_ref()
            .ok_or_else(|| ApplicationError::UnsupportedOperation {
                operation: "semantic plan node detail read".to_owned(),
            })?;
    store
        .load_plan_node_details_at_snapshot(
            request.session,
            request.plan,
            request.node,
            request.snapshot,
            request.offset,
            request.limit,
        )
        .map_err(|message| ApplicationError::Internal { message })?
        .ok_or_else(|| ApplicationError::NotFound {
            subject: format!("plan snapshot {}", request.snapshot),
        })
}

macro_rules! application_plan_node_details_method {
    () => {
        /// Reads bounded gates and evidence through the authorized semantic store.
        ///
        /// - `context`: Authorized request context.
        /// - `request`: Node, exact snapshot, and page range to inspect.
        ///
        /// # Returns
        /// The requested gates and associated evidence.
        ///
        /// # Errors
        /// Returns authorization, invalid-page, history, or persistence failures.
        fn read_plan_node_details(
            &self,
            context: &ApplicationContext,
            request: &crate::PlanNodeDetailsRequest,
        ) -> Result<scriberig_session::PlanSemanticNodeDetails, ApplicationError> {
            plan_node_details::read(self, context, request)
        }
    };
}
