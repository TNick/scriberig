//! Implements shared safe model-request detail reads.

use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService, SessionBackend,
    SessionRequest,
};
use scriberig_core::EntityId;

/// Loads the session and resolves one captured request by stable identities.
///
/// - `service`: Application service that owns session reads.
/// - `context`: Authorized request context.
/// - `session`: Session containing the invocation.
/// - `branch`: Branch containing the invocation.
/// - `invocation`: Stable model-invocation identity.
///
/// The captured safe request projection.
///
/// Returns authorization or lookup errors when detail is unavailable.
pub(super) fn model_request_detail<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
    branch: EntityId,
    invocation: EntityId,
) -> Result<scriberig_session::ModelRequestProjection, ApplicationError> {
    let session = SessionBackend::read_session(service, context, &SessionRequest { session })?;
    let branch = session
        .branches
        .iter()
        .find(|candidate| candidate.id == branch)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: branch.to_string(),
        })?;
    let model_invocation = branch
        .model_invocations
        .iter()
        .find(|candidate| candidate.id == invocation)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: invocation.to_string(),
        })?;
    model_invocation
        .request
        .clone()
        .ok_or_else(|| ApplicationError::NotFound {
            subject: invocation.to_string(),
        })
}
