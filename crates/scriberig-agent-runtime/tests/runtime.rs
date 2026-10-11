//! Verifies the provider/tool orchestration boundary.

use scriberig_agent_core::{InspectTool, ToolDetail, ToolInput};
use scriberig_agent_runtime::{
    AgentMessage, AgentResult, AgentRuntime, ModelDecision, ModelProvider,
};
use scriberig_app::{
    ApplicationContext, CapabilityContext, CapabilityId, InspectRequest, InspectResult,
    QueryService,
};
use scriberig_core::{EntityId, ObjectType, RevisionId, SnapshotId};
use std::sync::atomic::AtomicBool;

/// Provides one deterministic query service to the runtime test.
struct Fixture;

impl QueryService for Fixture {
    /// Returns one deterministic inspection.
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
    ) -> Result<InspectResult, scriberig_app::ApplicationError> {
        Ok(InspectResult {
            entity: request.entity,
            revision: Some(RevisionId::new([2; 32])),
            snapshot: Some(SnapshotId::new([3; 32])),
            object_type: Some(ObjectType::new("test.entity").expect("type")),
            label: Some("fixture".to_owned()),
            description: None,
            capabilities: vec!["inspect.entity".to_owned()],
            trust: None,
        })
    }
}

/// Provides one tool request followed by a final answer.
struct Provider {
    calls: usize,
}

impl ModelProvider for Provider {
    /// Produces a deterministic inspect call then final text.
    ///
    /// # Parameters
    /// - `messages`: Conversation messages and prior tool observations.
    ///
    /// # Returns
    /// The next inspect request or final response.
    ///
    /// # Errors
    /// Returns no provider error for this deterministic fixture.
    fn decide(&mut self, messages: &[AgentMessage]) -> Result<ModelDecision, String> {
        if self.calls == 0 {
            self.calls += 1;
            return Ok(ModelDecision::Tool(ToolInput {
                entity: EntityId::from_bytes([1; 16]),
                snapshot: None,
                detail: ToolDetail::Compact,
            }));
        }
        assert!(matches!(messages.last(), Some(AgentMessage::Tool(_))));
        Ok(ModelDecision::Final("done".to_owned()))
    }
}

/// Verifies the runtime executes tools only through agent-core contracts.
#[test]
fn runtime_completes_provider_tool_loop() {
    let registry = scriberig_app::OperationRegistry::new();
    let mut runtime = AgentRuntime::new(Provider { calls: 0 }, InspectTool::new(Fixture), 2)
        .with_operation_registry(registry.clone());
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("inspect.entity").expect("capability")
        ]),
        ..ApplicationContext::empty()
    };
    let result = runtime
        .run(&context, "inspect", &AtomicBool::new(false))
        .expect("agent result");
    assert_eq!(result.output, "done");
    assert_eq!(result.messages.len(), 2);
    let _: AgentResult = result;
    let events = registry.events();
    assert!(events.iter().any(|event| matches!(
        event,
        scriberig_app::ApplicationEvent::OperationCompleted { .. }
    )));
}
