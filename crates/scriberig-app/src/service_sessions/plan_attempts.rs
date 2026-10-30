//! Reads bounded attempt history through the authorized session repository.

use super::super::{ApplicationBackend, ApplicationService};
use super::authorize_session_read;
use crate::{ApplicationContext, ApplicationError, PlanAttemptHistoryRequest, SessionRequest};

/// Reads one bounded page of exact-snapshot worker attempts.
///
/// - `service`: Application service with its configured semantic store.
/// - `context`: Authorized request context.
/// - `request`: Node, exact snapshot, and page range to inspect.
///
/// Attempts ordered by node and generation.
/// # Returns
/// The requested attempt page.
/// # Errors
/// Returns authorization, invalid-page, history, or persistence failures.
pub(super) fn read<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    request: &PlanAttemptHistoryRequest,
) -> Result<Vec<scriberig_session::PlanSemanticAttempt>, ApplicationError> {
    authorize_session_read(
        context,
        &SessionRequest {
            session: request.session,
        },
    )?;
    if request.limit == 0 || request.limit > 256 {
        return Err(ApplicationError::InvalidRequest {
            operation: "plan attempt history read".to_owned(),
            message: "limit must be between 1 and 256".to_owned(),
        });
    }
    let store =
        service
            .session_store
            .as_ref()
            .ok_or_else(|| ApplicationError::UnsupportedOperation {
                operation: "semantic plan attempt history read".to_owned(),
            })?;
    store
        .load_plan_attempts_at_snapshot(
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
