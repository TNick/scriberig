//! Tests the module's deterministic behavior.

use super::{ApplicationAdapter, TuiService, execute_effect};
use crate::{action::Action, effect::Effect};
use scriberig_app::{
    ApplicationContext, ApplicationError, InspectRequest, InspectResult, QueryService,
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
                text: "term".to_owned()
            }
        ),
        Action::OperationFailed { category, .. } if category == "unsupported"
    ));
}
