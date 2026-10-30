//! Adapts typed plan requests to shared application operations.

#[path = "server_plan_operations/semantic_mutations.rs"]
mod semantic_mutations;
use semantic_mutations::semantic_mutation;

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService, SessionBackend,
};

/// Applies one typed plan request using application-owned validation.
///
/// - `service`: Shared application service.
/// - `context`: Trusted capability context.
/// - `request`: Typed plan operation request.
///
/// Safe projection of the authoritative plan state.
///
/// Returns authorization, validation, revision conflict, or persistence errors.
#[expect(
    clippy::too_many_lines,
    reason = "This dispatcher keeps the protocol request variants visibly mapped to application operations."
)]
pub(super) fn apply<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    request: &scriberig_protocol::Request,
) -> Result<scriberig_protocol::SessionPlan, ApplicationError> {
    let plan = match request {
        scriberig_protocol::Request::PlanCreate(request) => service.create_plan(
            context,
            &scriberig_app::PlanCreateRequest {
                session: parse_entity(&request.session.id)?,
                anchor: anchor(&request.anchor)?,
                goal: request.goal.clone(),
                nodes: request.nodes.iter().map(node_draft).collect(),
                dependencies: request.dependencies.clone(),
            },
        )?,
        scriberig_protocol::Request::PlanLifecycle(request) => {
            let app_request = scriberig_app::PlanLifecycleRequest {
                session: parse_entity(&request.session.id)?,
                plan: parse_entity(&request.plan)?,
                expected_revision: request.expected_revision,
            };
            request_action(service, context, &app_request, request.action)?
        }
        scriberig_protocol::Request::PlanAssignment(request) => service.set_plan_node_assignment(
            context,
            &scriberig_app::PlanAssignmentRequest {
                session: parse_entity(&request.session.id)?,
                plan: parse_entity(&request.plan)?,
                node: parse_entity(&request.node)?,
                expected_revision: request.expected_revision,
                assignment: request.assignment.clone().map(assignment),
            },
        )?,
        scriberig_protocol::Request::PlanReplan(request) => service.replan(
            context,
            &scriberig_app::PlanReplanRequest {
                session: parse_entity(&request.session.id)?,
                plan: parse_entity(&request.plan)?,
                expected_revision: request.expected_revision,
                additions: request.additions.iter().map(node_draft).collect(),
                dependencies: request
                    .dependencies
                    .iter()
                    .map(|(prerequisite, dependent)| {
                        Ok((node_reference(prerequisite)?, node_reference(dependent)?))
                    })
                    .collect::<Result<Vec<_>, ApplicationError>>()?,
                removed_dependencies: request
                    .removed_dependencies
                    .iter()
                    .map(|(prerequisite, dependent)| {
                        Ok((parse_entity(prerequisite)?, parse_entity(dependent)?))
                    })
                    .collect::<Result<Vec<_>, ApplicationError>>()?,
                superseded: request
                    .superseded
                    .iter()
                    .map(|(node, key)| Ok((parse_entity(node)?, key.clone())))
                    .collect::<Result<Vec<_>, ApplicationError>>()?,
                assignment_changes: request
                    .assignment_changes
                    .iter()
                    .map(|(node, value)| Ok((parse_entity(node)?, value.clone().map(assignment))))
                    .collect::<Result<Vec<_>, ApplicationError>>()?,
                preserved_terminal_nodes: request
                    .preserved_terminal_nodes
                    .iter()
                    .map(|node| parse_entity(node))
                    .collect::<Result<Vec<_>, _>>()?,
                reason: request.reason.clone(),
            },
        )?,
        scriberig_protocol::Request::PlanRetry(request) => service.retry_plan_node(
            context,
            &scriberig_app::PlanRetryRequest {
                session: parse_entity(&request.session.id)?,
                plan: parse_entity(&request.plan)?,
                node: parse_entity(&request.node)?,
                expected_revision: request.expected_revision,
            },
        )?,
        scriberig_protocol::Request::PlanNextStep(request) => service.add_plan_next_step(
            context,
            &scriberig_app::PlanNextStepRequest {
                session: parse_entity(&request.session.id)?,
                plan: parse_entity(&request.plan)?,
                expected_revision: request.expected_revision,
                title: request.title.clone(),
                purpose: request.purpose.clone(),
                expected_result: request.expected_result.as_ref().map(|expected| {
                    scriberig_session::ExpectedResult {
                        kind: expected.kind.clone(),
                        requires_handle: expected.requires_handle,
                    }
                }),
                semantic_scope: request.semantic_scope.clone(),
                assignment: request.assignment.clone().map(assignment),
            },
        )?,
        scriberig_protocol::Request::PlanDecompose(request) => service.decompose_plan_node(
            context,
            &scriberig_app::PlanDecomposeRequest {
                session: parse_entity(&request.session.id)?,
                plan: parse_entity(&request.plan)?,
                parent: parse_entity(&request.parent)?,
                expected_revision: request.expected_revision,
                children: request
                    .children
                    .iter()
                    .map(|child| scriberig_app::PlanChildDraft {
                        title: child.title.clone(),
                        purpose: child.purpose.clone(),
                        expected_result: child.expected_result.as_ref().map(|expected| {
                            scriberig_session::ExpectedResult {
                                kind: expected.kind.clone(),
                                requires_handle: expected.requires_handle,
                            }
                        }),
                        semantic_scope: child.semantic_scope.clone(),
                        assignment: child.assignment.clone().map(assignment),
                    })
                    .collect(),
                required: request.required,
            },
        )?,
        scriberig_protocol::Request::PlanSemanticMutation(request) => service
            .mutate_plan_semantics(
                context,
                &scriberig_app::PlanSemanticMutationRequest {
                    session: parse_entity(&request.session.id)?,
                    plan: parse_entity(&request.plan)?,
                    expected_revision: request.expected_revision,
                    mutation: semantic_mutation(request.mutation.clone())?,
                },
            )?,
        _ => {
            return Err(ApplicationError::InvalidRequest {
                operation: "plan".to_owned(),
                message: "request is not a plan operation".to_owned(),
            });
        }
    };
    let session_id = match request {
        scriberig_protocol::Request::PlanCreate(request) => parse_entity(&request.session.id)?,
        scriberig_protocol::Request::PlanLifecycle(request) => parse_entity(&request.session.id)?,
        scriberig_protocol::Request::PlanAssignment(request) => parse_entity(&request.session.id)?,
        scriberig_protocol::Request::PlanReplan(request) => parse_entity(&request.session.id)?,
        scriberig_protocol::Request::PlanRetry(request) => parse_entity(&request.session.id)?,
        scriberig_protocol::Request::PlanNextStep(request) => parse_entity(&request.session.id)?,
        scriberig_protocol::Request::PlanDecompose(request) => parse_entity(&request.session.id)?,
        scriberig_protocol::Request::PlanSemanticMutation(request) => {
            parse_entity(&request.session.id)?
        }
        _ => {
            return Err(ApplicationError::InvalidRequest {
                operation: "plan".to_owned(),
                message: "request is not a plan operation".to_owned(),
            });
        }
    };
    let session = service.read_session(
        context,
        &scriberig_app::SessionRequest {
            session: session_id,
        },
    )?;
    let results = session
        .branches
        .iter()
        .filter_map(|branch| branch.result.clone())
        .collect::<Vec<_>>();
    super::server_helpers::session_plan::project(plan, Some(&results))
}

/// Applies the requested plan lifecycle operation.
///
/// - `service`: Shared application service.
/// - `context`: Trusted caller context.
/// - `request`: Stable plan and expected revision.
/// - `action`: Typed lifecycle operation.
///
/// The updated durable plan.
///
/// Returns conflict, invalid lifecycle, or persistence failures.
fn request_action<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    request: &scriberig_app::PlanLifecycleRequest,
    action: scriberig_protocol::SessionPlanAction,
) -> Result<scriberig_session::Plan, ApplicationError> {
    match action {
        scriberig_protocol::SessionPlanAction::Start => service.start_plan(context, request),
        scriberig_protocol::SessionPlanAction::Pause => service.pause_plan(context, request),
        scriberig_protocol::SessionPlanAction::Resume => service.resume_plan(context, request),
        scriberig_protocol::SessionPlanAction::Stop => service.stop_plan(context, request),
        scriberig_protocol::SessionPlanAction::Complete => service.complete_plan(context, request),
    }
}

/// Converts one protocol node draft to application input.
///
/// - `value`: Provider-neutral structured proposal node.
///
/// Untrusted draft validated by the application service.
/// # Returns
/// The operation result.
fn node_draft(value: &scriberig_protocol::SessionPlanNodeDraft) -> scriberig_app::PlanNodeDraft {
    scriberig_app::PlanNodeDraft {
        key: value.key.clone(),
        parent_index: value.parent_index,
        title: value.title.clone(),
        purpose: value.purpose.clone(),
        expected_result: value.expected_result.as_ref().map(|expected| {
            scriberig_session::ExpectedResult {
                kind: expected.kind.clone(),
                requires_handle: expected.requires_handle,
            }
        }),
        semantic_scope: value.semantic_scope.clone(),
        assignment: value.assignment.clone().map(assignment),
    }
}

/// Converts one safe model assignment.
///
/// - `value`: Credential-free provider, model, and thinking identifiers.
///
/// Shared application assignment.
/// # Returns
/// The operation result.
fn assignment(
    value: scriberig_protocol::SessionModelAssignment,
) -> scriberig_session::ModelAssignment {
    scriberig_session::ModelAssignment {
        provider_id: value.provider_id,
        model_id: value.model_id,
        thinking_level: value.thinking_level,
    }
}

/// Converts one protocol node reference to application identity.
///
/// - `value`: Existing node or request-local proposed key.
///
/// Shared application node reference.
///
/// Returns invalid-request when an existing identity is malformed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn node_reference(
    value: &scriberig_protocol::SessionPlanNodeReference,
) -> Result<scriberig_app::PlanNodeReference, ApplicationError> {
    Ok(match value {
        scriberig_protocol::SessionPlanNodeReference::Existing { id } => {
            scriberig_app::PlanNodeReference::Existing(parse_entity(id)?)
        }
        scriberig_protocol::SessionPlanNodeReference::Proposed { key } => {
            scriberig_app::PlanNodeReference::Proposed(key.clone())
        }
    })
}

/// Converts a stable protocol identity to a shared entity id.
///
/// - `value`: Serialized stable identity.
///
/// Typed entity identity.
///
/// Returns invalid-request when the identity is malformed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn parse_entity(value: &str) -> Result<scriberig_core::EntityId, ApplicationError> {
    value
        .parse()
        .map_err(|error| ApplicationError::InvalidRequest {
            operation: "plan".to_owned(),
            message: format!("invalid plan identity: {error}"),
        })
}

/// Converts a protocol continuation anchor to shared session identity.
///
/// - `value`: Exact provider-neutral continuation anchor.
///
/// Session-domain continuation anchor.
///
/// Returns invalid-request when an anchor identity is malformed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn anchor(
    value: &scriberig_protocol::SessionContinuationAnchor,
) -> Result<scriberig_session::ContinuationAnchor, ApplicationError> {
    Ok(match value {
        scriberig_protocol::SessionContinuationAnchor::BranchHead { branch } => {
            scriberig_session::ContinuationAnchor::BranchHead {
                branch: parse_entity(branch)?,
            }
        }
        scriberig_protocol::SessionContinuationAnchor::AssistantOutput { branch, turn } => {
            scriberig_session::ContinuationAnchor::AssistantOutput {
                branch: parse_entity(branch)?,
                turn: parse_entity(turn)?,
            }
        }
        scriberig_protocol::SessionContinuationAnchor::HumanInput { branch, turn } => {
            scriberig_session::ContinuationAnchor::HumanInput {
                branch: parse_entity(branch)?,
                turn: parse_entity(turn)?,
            }
        }
        scriberig_protocol::SessionContinuationAnchor::BranchResult { branch, result } => {
            scriberig_session::ContinuationAnchor::BranchResult {
                branch: parse_entity(branch)?,
                result: parse_entity(result)?,
            }
        }
    })
}

macro_rules! shared_plan_operation_methods {
    () => {
        /// Applies typed plan operations through the shared application layer.
        /// # Parameters
        /// - `context`: Input for this operation.
        /// - `request`: Input for this operation.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn plan_operation(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::Request,
        ) -> Result<scriberig_protocol::SessionPlan, ApplicationError> {
            super::server_plan_operations::apply(self, context, request)
        }
    };
}
