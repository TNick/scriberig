//! Defines the narrow application-facing port used by TUI semantic views.

use crate::{action::Action, effect::Effect, inspector::InspectorModel};
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService, InspectRequest, InspectResult, QueryService,
    RuntimeCapability, SnapshotResolution,
};
use scriberig_core::{EntityId, SnapshotId};

/// Supplies semantic reads to the TUI without exposing storage internals.
pub trait TuiService {
    /// Inspects one entity through the shared application contract.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The application inspection result.
    ///
    /// # Errors
    /// Returns the structured application error.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError>;

    /// Returns runtime capabilities exposed by the application composition.
    ///
    /// # Parameters
    /// - `_context`: Application context for capability resolution.
    ///
    /// # Returns
    /// The capabilities available to the TUI.
    ///
    /// # Errors
    /// Returns the structured application error.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        Ok(Vec::new())
    }

    /// Executes one shared application operation when the adapter supports it.
    fn execute_operation(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "tui semantic operation".to_owned(),
        })
    }
}

/// Supplies shared application operations to semantic TUI views.
pub trait TuiOperationService {
    /// Executes one public application operation without exposing storage.
    ///
    /// # Parameters
    /// - `context`: Authorized application context.
    /// - `request`: Public universal application request.
    ///
    /// # Returns
    /// Structured application result owned by the application layer.
    ///
    /// # Errors
    /// Returns the structured application error from the owning subsystem.
    fn execute_operation(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError>;
}

/// Adapts the complete public application facade for operation-backed views.
pub struct ApplicationOperationAdapter<B> {
    service: ApplicationService<B>,
}

impl<B> ApplicationOperationAdapter<B> {
    /// Creates an operation adapter around an application service.
    #[must_use]
    pub const fn new(service: ApplicationService<B>) -> Self {
        Self { service }
    }
}

impl<B: ApplicationBackend> TuiService for ApplicationOperationAdapter<B> {
    /// Delegates inspection through the public application facade.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        self.service.inspect(
            context,
            &InspectRequest {
                entity,
                snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
                detail: scriberig_app::DetailLevel::Detailed,
            },
        )
    }

    /// Delegates shared operations through the public application facade.
    fn execute_operation(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        self.service.execute(context, request)
    }
}

impl<S: TuiService> TuiOperationService for S {
    /// Uses the operation method exposed by the TUI service boundary.
    fn execute_operation(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        TuiService::execute_operation(self, context, request)
    }
}

/// Executes one public operation through the TUI operation port.
///
/// # Parameters
/// - `service`: TUI operation service.
/// - `context`: Application capability context.
/// - `request`: Operation request to execute.
///
/// # Returns
/// The application operation result.
///
/// # Errors
/// Returns the application error produced by the operation service.
pub fn execute_operation<S: TuiOperationService>(
    service: &S,
    context: &ApplicationContext,
    request: &ApplicationRequest,
) -> Result<ApplicationResult, ApplicationError> {
    service.execute_operation(context, request)
}

/// Executes a non-inspection effect and converts its public result to an action.
pub fn execute_operation_effect<S: TuiOperationService>(
    service: &S,
    context: &ApplicationContext,
    effect: Effect,
) -> Action {
    let request_id = match &effect {
        Effect::History { request_id, .. }
        | Effect::Diff { request_id, .. }
        | Effect::Render { request_id, .. }
        | Effect::Apply { request_id, .. }
        | Effect::Commit { request_id, .. } => *request_id,
        _ => return Action::RefreshRequested,
    };
    let request = match effect {
        Effect::History { snapshot, .. } => {
            scriberig_app::ApplicationRequest::History(scriberig_app::HistoryRequest {
                snapshot: scriberig_app::SnapshotSelector::Exact(snapshot),
            })
        }
        Effect::Diff { request, .. } => scriberig_app::ApplicationRequest::Diff(request),
        Effect::Render { request, .. } => scriberig_app::ApplicationRequest::Render(request),
        Effect::Apply { request, .. } => scriberig_app::ApplicationRequest::Apply(request),
        Effect::Commit { request, .. } => scriberig_app::ApplicationRequest::Commit(request),
        _ => unreachable!(),
    };
    match service.execute_operation(context, &request) {
        Ok(scriberig_app::ApplicationResult::History(summary)) => {
            let snapshot = summary.snapshot.unwrap_or_else(|| match &request {
                scriberig_app::ApplicationRequest::History(value) => match value.snapshot {
                    scriberig_app::SnapshotSelector::Exact(snapshot) => snapshot,
                    scriberig_app::SnapshotSelector::NamedRoot(_) => {
                        scriberig_core::SnapshotId::new([0; 32])
                    }
                },
                _ => scriberig_core::SnapshotId::new([0; 32]),
            });
            Action::HistorySucceeded {
                request_id,
                model: crate::view::history::HistoryModel::from_summary(snapshot, &summary),
            }
        }
        Ok(scriberig_app::ApplicationResult::Diff(summary)) => summary.diff.as_ref().map_or_else(
            || Action::OperationFailed {
                request_id,
                category: "invalid_result".to_owned(),
                message: "diff result did not contain semantic changes".to_owned(),
            },
            |diff| Action::DiffSucceeded {
                request_id,
                model: crate::view::diff::DiffModel::from_diff(diff),
            },
        ),
        Ok(scriberig_app::ApplicationResult::Render(summary)) => {
            summary.rendered.as_ref().map_or_else(
                || Action::OperationFailed {
                    request_id,
                    category: "invalid_result".to_owned(),
                    message: "render result did not contain projection content".to_owned(),
                },
                |rendered| Action::ProjectionSucceeded {
                    request_id,
                    model: crate::view::projection::ProjectionModel::from_rendered(rendered),
                },
            )
        }
        Ok(scriberig_app::ApplicationResult::Apply(summary)) => {
            let scriberig_app::ApplicationRequest::Apply(request) = request else {
                unreachable!()
            };
            Action::ReviewSucceeded {
                request_id,
                model: crate::view::changes::ChangeReviewModel::from_summary(
                    request.base_snapshot,
                    request.change,
                    request.branch,
                    &summary,
                ),
            }
        }
        Ok(scriberig_app::ApplicationResult::Commit(summary)) => {
            let scriberig_app::ApplicationRequest::Commit(request) = request else {
                unreachable!()
            };
            Action::ReviewSucceeded {
                request_id,
                model: crate::view::changes::ChangeReviewModel::from_summary(
                    request.base_snapshot,
                    request.change,
                    request.branch,
                    &summary,
                ),
            }
        }
        Ok(_) => Action::OperationFailed {
            request_id,
            category: "invalid_result".to_owned(),
            message: "application returned an unexpected operation result".to_owned(),
        },
        Err(error) => Action::OperationFailed {
            request_id,
            category: error.category().to_owned(),
            message: crate::inspector::sanitize(&error.to_string()),
        },
    }
}

/// Adapts any public application query service to the TUI port.
pub struct ApplicationAdapter<S> {
    service: S,
    capabilities: Vec<RuntimeCapability>,
}

impl<S> ApplicationAdapter<S> {
    /// Creates a TUI adapter around a public application service.
    #[must_use]
    pub fn new(service: S, capabilities: Vec<RuntimeCapability>) -> Self {
        Self {
            service,
            capabilities,
        }
    }

    /// Borrows the wrapped application service.
    #[must_use]
    pub const fn service(&self) -> &S {
        &self.service
    }
}

impl<S: QueryService> TuiService for ApplicationAdapter<S> {
    /// Delegates inspection through the public query service.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        self.service.inspect(
            context,
            &InspectRequest {
                entity,
                snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
                detail: scriberig_app::DetailLevel::Detailed,
            },
        )
    }

    /// Returns the capabilities supplied by the composition root.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        Ok(self.capabilities.clone())
    }
}

impl<S: QueryService> TuiService for S {
    /// Delegates inspection through a public query service.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        self.inspect(
            context,
            &InspectRequest {
                entity,
                snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
                detail: scriberig_app::DetailLevel::Detailed,
            },
        )
    }
}

/// Executes one semantic effect outside the pure reducer boundary.
#[allow(clippy::needless_pass_by_value)]
pub fn execute_effect<S: TuiService>(
    service: &S,
    context: &ApplicationContext,
    effect: Effect,
) -> Action {
    match effect {
        Effect::InspectEntity {
            request_id,
            entity,
            snapshot,
        } => match service.inspect_entity(context, entity, snapshot) {
            Ok(result) => Action::InspectSucceeded {
                request_id,
                model: InspectorModel::from_result(result),
            },
            Err(error) => Action::InspectFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            },
        },
        Effect::RequestCapabilities => match service.runtime_capabilities(context) {
            Ok(capabilities) => Action::CapabilitiesLoaded(capabilities),
            Err(error) => Action::InspectFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            },
        },
        Effect::History { .. }
        | Effect::Diff { .. }
        | Effect::Render { .. }
        | Effect::Apply { .. }
        | Effect::Commit { .. } => execute_operation_effect(service, context, effect),
        _ => Action::RefreshRequested,
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplicationAdapter, TuiService};
    use scriberig_app::{
        ApplicationContext, ApplicationError, InspectRequest, InspectResult, QueryService,
    };
    use scriberig_core::{EntityId, SnapshotId};

    struct Fixture;

    impl QueryService for Fixture {
        /// Returns a deterministic public inspection result.
        fn inspect(
            &self,
            _context: &ApplicationContext,
            request: &InspectRequest,
        ) -> Result<InspectResult, ApplicationError> {
            Ok(InspectResult {
                entity: request.entity,
                revision: None,
                snapshot: match request.snapshot {
                    scriberig_app::SnapshotResolution::At(snapshot) => Some(snapshot),
                    _ => None,
                },
                object_type: None,
                label: Some("fixture".to_owned()),
                description: None,
                capabilities: Vec::new(),
                trust: None,
            })
        }
    }

    /// Verifies the adapter preserves exact public snapshot context.
    #[test]
    fn delegates_public_inspection_contract() {
        let adapter = ApplicationAdapter::new(Fixture, Vec::new());
        let entity = EntityId::from_bytes([7; 16]);
        let snapshot = SnapshotId::new([8; 32]);
        let result = adapter
            .inspect_entity(&ApplicationContext::empty(), entity, Some(snapshot))
            .expect("inspection");
        assert_eq!(result.entity, entity);
        assert_eq!(result.snapshot, Some(snapshot));
    }
}
