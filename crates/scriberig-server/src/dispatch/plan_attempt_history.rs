//! Dispatches bounded plan attempt-history reads.

use std::str::FromStr;

use super::{ApplicationService, Dispatcher, Response, command_context, helpers};
use scriberig_app::{ApplicationError, PlanAttemptHistoryRequest};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_protocol::{
    SessionPlanAttempt, SessionPlanAttemptHistoryPage, SessionPlanEvidenceKind,
    SessionPlanEvidenceReference, SessionPlanNodeStatus, SessionSemanticReference,
};

impl<S: ApplicationService> Dispatcher<S> {
    /// Reads exact-snapshot worker attempts and projects the bounded page.
    ///
    /// - `request`: Typed protocol query with string identities.
    /// - `correlation_id`: Request correlation identity.
    ///
    /// Bounded attempt page or structured protocol error.
    /// # Returns
    /// The protocol response.
    pub(super) fn plan_attempt_history(
        &self,
        request: &scriberig_protocol::SessionPlanAttemptHistoryRequest,
        correlation_id: &str,
    ) -> Response {
        let app_request = (|| {
            Ok(PlanAttemptHistoryRequest {
                session: parse_entity(&request.session.id, "session")?,
                plan: parse_entity(&request.plan, "plan")?,
                node: parse_entity(&request.node, "node")?,
                snapshot: SnapshotId::from_str(&request.snapshot)
                    .map_err(|error| invalid_identity("snapshot", &error.to_string()))?,
                offset: request.offset,
                limit: request.limit,
            })
        })();
        let attempts = match app_request.and_then(|app_request| {
            self.service
                .read_plan_attempt_history(&command_context("session_branch_tree"), &app_request)
        }) {
            Ok(attempts) => attempts,
            Err(error) => return Response::Error(helpers::map_error(&error, correlation_id)),
        };
        Response::PlanAttemptHistory(SessionPlanAttemptHistoryPage {
            offset: request.offset,
            attempts: attempts.into_iter().map(project_attempt).collect(),
        })
    }
}

/// Parses one stable entity identity from a protocol string.
///
/// - `value`: String-encoded entity identity.
/// - `field`: Request field name for diagnostics.
///
/// The parsed identity.
/// # Returns
/// The parsed value.
/// # Errors
/// Returns an invalid-request error for malformed identities.
fn parse_entity(value: &str, field: &str) -> Result<EntityId, ApplicationError> {
    EntityId::from_str(value).map_err(|error| invalid_identity(field, &error.to_string()))
}

/// Creates a structured invalid-identity application error.
///
/// - `field`: Request field name.
/// - `message`: Safe parser diagnostic.
///
/// An invalid-request application error.
/// # Returns
/// The structured invalid-request error.
fn invalid_identity(field: &str, message: &str) -> ApplicationError {
    ApplicationError::InvalidRequest {
        operation: "plan attempt history read".to_owned(),
        message: format!("invalid {field} identity: {message}"),
    }
}

/// Projects one semantic attempt into protocol-safe identities.
///
/// - `attempt`: Validated exact-snapshot semantic attempt.
///
/// Protocol attempt record without storage implementation details.
/// # Returns
/// The protocol attempt record.
fn project_attempt(attempt: scriberig_session::PlanSemanticAttempt) -> SessionPlanAttempt {
    SessionPlanAttempt {
        id: attempt.entity.to_string(),
        exact_revision: attempt.revision.to_string(),
        node: attempt.node.to_string(),
        branch: attempt.branch.to_string(),
        claim_generation: attempt.claim_generation,
        claim: attempt.claim.map(|claim| claim.to_string()),
        status: match attempt.status {
            scriberig_session::PlanNodeStatus::Planned => SessionPlanNodeStatus::Planned,
            scriberig_session::PlanNodeStatus::Ready => SessionPlanNodeStatus::Ready,
            scriberig_session::PlanNodeStatus::Running => SessionPlanNodeStatus::Running,
            scriberig_session::PlanNodeStatus::Completed => SessionPlanNodeStatus::Completed,
            scriberig_session::PlanNodeStatus::Failed => SessionPlanNodeStatus::Failed,
            scriberig_session::PlanNodeStatus::Abandoned => SessionPlanNodeStatus::Abandoned,
        },
        result: attempt.result.map(|result| result.to_string()),
        failure: attempt.failure.map(project_failure),
    }
}

/// Projects exact failure evidence into its protocol representation.
///
/// - `failure`: Semantic failure reference retained by the attempt.
///
/// Protocol evidence reference with exact identities.
/// # Returns
/// The protocol evidence reference.
fn project_failure(
    failure: scriberig_session::PlanEvidenceReference,
) -> SessionPlanEvidenceReference {
    SessionPlanEvidenceReference {
        kind: match failure.kind {
            scriberig_session::PlanEvidenceKind::BranchResult => {
                SessionPlanEvidenceKind::BranchResult
            }
            scriberig_session::PlanEvidenceKind::ValidationResult => {
                SessionPlanEvidenceKind::ValidationResult
            }
            scriberig_session::PlanEvidenceKind::Decision => SessionPlanEvidenceKind::Decision,
            scriberig_session::PlanEvidenceKind::HumanApproval => {
                SessionPlanEvidenceKind::HumanApproval
            }
            scriberig_session::PlanEvidenceKind::Candidate => SessionPlanEvidenceKind::Candidate,
            scriberig_session::PlanEvidenceKind::Artifact => SessionPlanEvidenceKind::Artifact,
            scriberig_session::PlanEvidenceKind::Failure => SessionPlanEvidenceKind::Failure,
            scriberig_session::PlanEvidenceKind::Other(kind) => {
                SessionPlanEvidenceKind::Other(kind)
            }
        },
        reference: SessionSemanticReference {
            entity: failure.reference.entity.to_string(),
            revision: failure
                .reference
                .revision
                .map(|revision| revision.to_string()),
            snapshot: failure
                .reference
                .snapshot
                .map(|snapshot| snapshot.to_string()),
        },
        attributes: failure.attributes,
    }
}
