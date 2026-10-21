//! Tests the module's deterministic behavior.

use super::{ApplicationAdapter, TuiService, execute_effect};
use crate::{action::Action, effect::Effect};
use scriberig_app::{
    ApplicationContext, ApplicationError, InspectRequest, InspectResult, QueryService,
    SessionBackend,
};
use scriberig_core::{EntityId, SnapshotId};

/// Provides a deterministic query result for adapter tests.
struct Fixture;

impl QueryService for Fixture {
    /// Returns a deterministic public inspection result.
    ///
    /// - `_context`: Application context for the inspection.
    /// - `request`: Inspection request to resolve.
    ///
    /// The deterministic inspection result.
    ///
    /// This fixture does not return an application error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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

impl SessionBackend for Fixture {
    /// Rejects session reads in the query-only test fixture.
    ///
    /// - `_context`: Application context.
    /// - `_request`: Session read request.
    ///
    /// Never returns a session.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_session(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::SessionRequest,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test session read".to_owned(),
        })
    }

    /// Returns a deterministic plan for shared adapter routing.
    ///
    /// - `_context`: Application request context.
    /// - `request`: Structured plan proposal.
    ///
    /// A valid plan carrying the requested goal and anchor.
    ///
    /// Returns an error when the fixture proposal cannot form a plan.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn create_plan(
        &self,
        _context: &ApplicationContext,
        request: &scriberig_app::PlanCreateRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        let mut plan = scriberig_session::Plan::new(
            EntityId::from_bytes([91; 16]),
            request.session,
            request.anchor,
            request.goal.clone(),
        )
        .map_err(|error| ApplicationError::InvalidRequest {
            operation: "fixture plan".to_owned(),
            message: format!("{error:?}"),
        })?;
        plan.transition(scriberig_session::PlanStatus::Planned)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "fixture plan".to_owned(),
                message: format!("{error:?}"),
            })?;
        Ok(plan)
    }

    /// Returns the fixture plan after receiving a shared node assignment.
    ///
    /// - `_context`: Application request context.
    /// - `request`: Selected plan-node assignment.
    ///
    /// The authoritative fixture plan.
    ///
    /// Returns an error if the fixture plan cannot be constructed.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_plan_node_assignment(
        &self,
        _context: &ApplicationContext,
        request: &scriberig_app::PlanAssignmentRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        let mut plan = scriberig_session::Plan::new(
            request.plan,
            request.session,
            scriberig_session::ContinuationAnchor::BranchHead {
                branch: EntityId::from_bytes([92; 16]),
            },
            "fixture assignment".to_owned(),
        )
        .map_err(|error| ApplicationError::InvalidRequest {
            operation: "fixture assignment".to_owned(),
            message: format!("{error:?}"),
        })?;
        plan.transition(scriberig_session::PlanStatus::Planned)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "fixture assignment".to_owned(),
                message: format!("{error:?}"),
            })?;
        Ok(plan)
    }

    /// Applies a fixture replan through the shared adapter contract.
    ///
    /// - `_context`: Application request context.
    /// - `request`: Complete structured revision proposal.
    ///
    /// The authoritative fixture plan.
    ///
    /// Returns an error if the fixture plan cannot be constructed.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn replan(
        &self,
        _context: &ApplicationContext,
        request: &scriberig_app::PlanReplanRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        let mut plan = scriberig_session::Plan::new(
            request.plan,
            request.session,
            scriberig_session::ContinuationAnchor::BranchHead {
                branch: EntityId::from_bytes([96; 16]),
            },
            "fixture replan".to_owned(),
        )
        .map_err(|error| ApplicationError::InvalidRequest {
            operation: "fixture replan".to_owned(),
            message: format!("{error:?}"),
        })?;
        plan.transition(scriberig_session::PlanStatus::Planned)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "fixture replan".to_owned(),
                message: format!("{error:?}"),
            })?;
        Ok(plan)
    }

    /// Rejects agent commands in the query-only test fixture.
    ///
    /// - `_context`: Application context.
    /// - `_request`: Agent command request.
    ///
    /// Never returns an operation identity.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn submit_agent_command(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::AgentCommandRequest,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test agent command".to_owned(),
        })
    }
}

impl scriberig_app::AgentConsoleBackend for Fixture {
    /// Rejects model-library reads in this query-only fixture.
    ///
    /// - `_context`: Application request context.
    ///
    /// Never returns a model library.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn agent_model_library(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<rpi_model_runtime::SafeModelSnapshot>, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test model library".to_owned(),
        })
    }

    /// Rejects model selection in this query-only fixture.
    ///
    /// - `_context`: Application request context.
    /// - `_request`: Requested session model selection.
    ///
    /// Never returns an updated selection.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_session_model(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::SessionModelSelectionRequest,
    ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test model selection".to_owned(),
        })
    }

    /// Rejects thinking selection in this query-only fixture.
    ///
    /// - `_context`: Application request context.
    /// - `_request`: Requested session thinking selection.
    ///
    /// Never returns an updated selection.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_session_thinking_level(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::SessionThinkingSelectionRequest,
    ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test thinking selection".to_owned(),
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

/// Verifies control effects are translated into explicit reducer actions.
#[test]
fn translates_control_effects_without_silent_noops() {
    let service = ApplicationAdapter::new(Fixture, Vec::new());
    let context = ApplicationContext::empty();
    assert_eq!(
        execute_effect(&service, &context, Effect::RequestQuit),
        Action::QuitRequested
    );
    assert_eq!(
        execute_effect(&service, &context, Effect::RequestRefresh),
        Action::RefreshRequested
    );
    assert!(matches!(
        execute_effect(
            &service,
            &context,
            Effect::RequestFind {
                request_id: 1,
                text: "term".to_owned(),
                snapshot: SnapshotId::new([8; 32]),
            }
        ),
        Action::OperationFailed { category, .. } if category == "unsupported_operation"
    ));
}

/// Verifies node assignment effects use the shared application backend.
#[test]
fn delegates_plan_node_assignment_to_shared_session_backend() {
    let service = ApplicationAdapter::new(Fixture, Vec::new());
    let session = EntityId::from_bytes([93; 16]);
    let plan = EntityId::from_bytes([94; 16]);
    let node = EntityId::from_bytes([95; 16]);
    let action = execute_effect(
        &service,
        &ApplicationContext::empty(),
        Effect::SetPlanNodeAssignment(scriberig_app::PlanAssignmentRequest {
            session,
            plan,
            node,
            expected_revision: 2,
            assignment: None,
        }),
    );
    assert!(matches!(action, Action::PlanUpdated(value) if value.id == plan));
}

/// Verifies replan effects use the shared application backend.
#[test]
fn delegates_structured_plan_revision_to_shared_session_backend() {
    let service = ApplicationAdapter::new(Fixture, Vec::new());
    let session = EntityId::from_bytes([97; 16]);
    let plan = EntityId::from_bytes([98; 16]);
    let action = execute_effect(
        &service,
        &ApplicationContext::empty(),
        Effect::Replan(scriberig_app::PlanReplanRequest {
            session,
            plan,
            expected_revision: 1,
            additions: Vec::new(),
            dependencies: Vec::new(),
            removed_dependencies: Vec::new(),
            superseded: Vec::new(),
            assignment_changes: Vec::new(),
            preserved_terminal_nodes: Vec::new(),
            reason: "revise scope".to_owned(),
        }),
    );
    assert!(matches!(action, Action::PlanUpdated(value) if value.id == plan));
}

/// Routes plan creation effects through the application service adapter.
#[test]
fn executes_plan_creation_through_shared_service() {
    let service = ApplicationAdapter::new(Fixture, Vec::new());
    let request = scriberig_app::PlanCreateRequest {
        session: EntityId::from_bytes([92; 16]),
        anchor: scriberig_session::ContinuationAnchor::BranchHead {
            branch: EntityId::from_bytes([93; 16]),
        },
        goal: "Improve parser".to_owned(),
        nodes: Vec::new(),
        dependencies: Vec::new(),
    };

    let action = execute_effect(
        &service,
        &ApplicationContext::empty(),
        Effect::CreatePlan(request),
    );

    assert!(matches!(
        action,
        Action::PlanUpdated(plan) if plan.goal == "Improve parser"
    ));
}
