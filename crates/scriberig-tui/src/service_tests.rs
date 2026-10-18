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
    /// # Parameters
    /// - `_context`: Application context for the inspection.
    /// - `request`: Inspection request to resolve.
    ///
    /// # Returns
    /// The deterministic inspection result.
    ///
    /// # Errors
    /// This fixture does not return an application error.
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
    /// # Parameters
    /// - `_context`: Application context.
    /// - `_request`: Session read request.
    ///
    /// # Returns
    /// Never returns a session.
    ///
    /// # Errors
    /// Always returns an unsupported-operation error.
    fn read_session(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::SessionRequest,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test session read".to_owned(),
        })
    }

    /// Rejects agent commands in the query-only test fixture.
    ///
    /// # Parameters
    /// - `_context`: Application context.
    /// - `_request`: Agent command request.
    ///
    /// # Returns
    /// Never returns an operation identity.
    ///
    /// # Errors
    /// Always returns an unsupported-operation error.
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
    /// # Parameters
    /// - `_context`: Application request context.
    ///
    /// # Returns
    /// Never returns a model library.
    ///
    /// # Errors
    /// Always returns an unsupported-operation error.
    fn agent_model_library(
        &self,
        _context: &ApplicationContext,
    ) -> Result<scriberig_app::ModelLibrary, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test model library".to_owned(),
        })
    }

    /// Rejects model selection in this query-only fixture.
    ///
    /// # Parameters
    /// - `_context`: Application request context.
    /// - `_request`: Requested session model selection.
    ///
    /// # Returns
    /// Never returns an updated selection.
    ///
    /// # Errors
    /// Always returns an unsupported-operation error.
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
    /// # Parameters
    /// - `_context`: Application request context.
    /// - `_request`: Requested session thinking selection.
    ///
    /// # Returns
    /// Never returns an updated selection.
    ///
    /// # Errors
    /// Always returns an unsupported-operation error.
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
