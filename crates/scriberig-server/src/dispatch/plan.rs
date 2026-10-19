//! Dispatches typed plan lifecycle and assignment requests.

use super::{ApplicationService, Dispatcher, Request, Response, command_context, helpers};

impl<S: ApplicationService> Dispatcher<S> {
    /// Dispatches one typed plan request through the shared application layer.
    ///
    /// - `request`: Typed plan operation request.
    /// - `correlation_id`: Request correlation identity.
    /// - `plan_lifecycle_handler`: Optional trusted plan runtime.
    ///
    /// Updated safe plan projection or a structured protocol error.
    /// # Returns
    /// The operation result.
    pub(super) fn plan_request(
        &self,
        request: &Request,
        correlation_id: &str,
        plan_lifecycle_handler: Option<&crate::http::PlanLifecycleHandler<S>>,
    ) -> Response {
        if !self
            .service
            .capabilities()
            .iter()
            .any(|capability| capability.id.as_str() == "agent_plan" && capability.available)
        {
            return Response::Error(helpers::map_error(
                &scriberig_app::ApplicationError::UnsupportedOperation {
                    operation: "agent_plan".to_owned(),
                },
                correlation_id,
            ));
        }
        let context = command_context("agent_plan");
        let result = match (request, plan_lifecycle_handler) {
            (Request::PlanLifecycle(request), Some(handler)) => {
                use std::str::FromStr;
                let app_request = scriberig_app::PlanLifecycleRequest {
                    session: match scriberig_core::EntityId::from_str(&request.session.id) {
                        Ok(value) => value,
                        Err(error) => {
                            return Response::Error(helpers::map_error(
                                &scriberig_app::ApplicationError::InvalidRequest {
                                    operation: "plan lifecycle".to_owned(),
                                    message: error.to_string(),
                                },
                                correlation_id,
                            ));
                        }
                    },
                    plan: match scriberig_core::EntityId::from_str(&request.plan) {
                        Ok(value) => value,
                        Err(error) => {
                            return Response::Error(helpers::map_error(
                                &scriberig_app::ApplicationError::InvalidRequest {
                                    operation: "plan lifecycle".to_owned(),
                                    message: error.to_string(),
                                },
                                correlation_id,
                            ));
                        }
                    },
                    expected_revision: request.expected_revision,
                };
                let action = match request.action {
                    scriberig_protocol::SessionPlanAction::Start => {
                        scriberig_app::PlanLifecycleAction::Start
                    }
                    scriberig_protocol::SessionPlanAction::Pause => {
                        scriberig_app::PlanLifecycleAction::Pause
                    }
                    scriberig_protocol::SessionPlanAction::Resume => {
                        scriberig_app::PlanLifecycleAction::Resume
                    }
                    scriberig_protocol::SessionPlanAction::Stop => {
                        scriberig_app::PlanLifecycleAction::Stop
                    }
                    scriberig_protocol::SessionPlanAction::Complete => {
                        scriberig_app::PlanLifecycleAction::Complete
                    }
                };
                handler(&self.service, &context, &app_request, action)
                    .map(crate::server::server_helpers::project_session_plan)
            }
            _ => self.service.plan_operation(&context, request),
        };
        result.map_or_else(
            |error| Response::Error(helpers::map_error(&error, correlation_id)),
            Response::Plan,
        )
    }
}
