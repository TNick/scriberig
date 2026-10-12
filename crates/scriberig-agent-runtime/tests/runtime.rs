//! Verifies the provider/tool orchestration boundary.

use scriberig_agent_core::{InspectTool, Tool, ToolDetail, ToolError, ToolInput, ToolResult};
use scriberig_agent_runtime::{
    AgentMessage, AgentResult, AgentRuntime, ChildExecutor, ChildRequest, ChildResult,
    EchoProvider, ModelDecision, ModelProvider, run_local_agent, run_local_one_shot,
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

/// Verifies child results are projected into the parent loop as compact data.
#[test]
fn child_executor_is_an_explicit_runtime_boundary() {
    /// Child executor fixture returning a compact result.
    struct Fixture;

    impl ChildExecutor for Fixture {
        /// Verifies the scope and returns one compact child result.
        ///
        /// # Parameters
        /// - `_context`: Test application context.
        /// - `scope`: Parent branch scope.
        /// - `request`: Child request.
        /// - `_cancelled`: Test cancellation flag.
        ///
        /// # Returns
        /// Compact fixture child result.
        ///
        /// # Errors
        /// This fixture never returns an error.
        fn execute_child(
            &mut self,
            _context: &scriberig_app::ApplicationContext,
            scope: Option<scriberig_agent_runtime::ChildExecutionScope>,
            request: ChildRequest,
            _cancelled: &std::sync::atomic::AtomicBool,
        ) -> Result<ChildResult, String> {
            assert!(scope.is_some());
            assert_eq!(request.purpose, "inspect");
            Ok(ChildResult {
                summary: "child complete".to_owned(),
                handles: vec!["ent_result".to_owned()],
            })
        }
    }

    /// Provider fixture requesting exactly one child.
    struct Provider;

    impl ModelProvider for Provider {
        /// Returns a child decision followed by a final response.
        ///
        /// # Parameters
        /// - `messages`: Parent conversation messages.
        ///
        /// # Returns
        /// The next scripted decision.
        ///
        /// # Errors
        /// This fixture never returns an error.
        fn decide(&mut self, messages: &[AgentMessage]) -> Result<ModelDecision, String> {
            if messages.len() == 1 {
                return Ok(ModelDecision::Child(ChildRequest {
                    purpose: "inspect".to_owned(),
                    prompt: "inspect the selected entity".to_owned(),
                }));
            }
            assert!(matches!(messages[1], AgentMessage::Child(_)));
            Ok(ModelDecision::Final("parent complete".to_owned()))
        }
    }

    let scope = scriberig_agent_runtime::ChildExecutionScope {
        session: scriberig_core::EntityId::from_bytes([8; 16]),
        parent_branch: scriberig_core::EntityId::from_bytes([9; 16]),
        depth: 0,
        active_branches: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let mut runtime = AgentRuntime::new(Provider, scriberig_agent_runtime::UnconfiguredTool, 2);
    let mut children = Fixture;
    let result = runtime
        .run_with_child_executor_at(
            &scope,
            &scriberig_app::ApplicationContext::empty(),
            "parent",
            &std::sync::atomic::AtomicBool::new(false),
            &mut children,
        )
        .expect("parent runtime");
    assert_eq!(result.output, "parent complete");
    assert!(matches!(result.messages[1], AgentMessage::Child(_)));
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

/// Verifies one-shot execution uses the shared agent runtime path.
#[test]
fn local_one_shot_uses_agent_runtime() {
    let result = run_local_one_shot("hello").expect("one-shot result");
    assert_eq!(result.output, "hello");
    assert_eq!(
        result.messages,
        vec![AgentMessage::User("hello".to_owned())]
    );
}

/// Verifies the agent command uses the agent runtime boundary.
#[test]
fn local_agent_returns_prompt_through_agent_runtime() {
    let result = run_local_agent("hello").expect("agent result");
    assert_eq!(result.output, "hello");
    assert_eq!(
        result.messages,
        vec![AgentMessage::User("hello".to_owned())]
    );
}

/// Verifies cancellation stops the shared runtime before provider work begins.
#[test]
fn runtime_reports_cancellation_to_shared_registry() {
    let registry = scriberig_app::OperationRegistry::new();
    let mut runtime = AgentRuntime::new(EchoProvider, scriberig_agent_runtime::UnconfiguredTool, 1)
        .with_operation_registry(registry.clone());
    let cancelled = AtomicBool::new(true);

    let error = runtime
        .run(&ApplicationContext::empty(), "cancel", &cancelled)
        .expect_err("cancelled runtime");

    assert_eq!(error, scriberig_agent_runtime::AgentError::Cancelled);
    assert!(registry.events().iter().any(|event| matches!(
        event,
        scriberig_app::ApplicationEvent::OperationCancelled { .. }
    )));
}

/// Verifies the runtime enforces the configured tool-call budget.
#[test]
fn runtime_enforces_tool_call_budget() {
    let mut runtime = AgentRuntime::new(RepeatedToolProvider, NoopTool, 8).with_limits(
        scriberig_agent_runtime::orchestration::RuntimeLimits {
            max_tool_calls: 1,
            ..Default::default()
        },
    );

    let error = runtime
        .run(
            &ApplicationContext::empty(),
            "budget",
            &AtomicBool::new(false),
        )
        .expect_err("second tool call must exceed the budget");

    assert!(error.to_string().contains("tool call budget"));
}

/// Verifies an overlong provider response is converted to a budget failure.
#[test]
fn runtime_enforces_provider_duration_budget() {
    let mut runtime = AgentRuntime::new(SlowProvider, NoopTool, 1).with_limits(
        scriberig_agent_runtime::orchestration::RuntimeLimits {
            max_provider_duration: std::time::Duration::ZERO,
            ..Default::default()
        },
    );
    let error = runtime
        .run(
            &ApplicationContext::empty(),
            "slow",
            &AtomicBool::new(false),
        )
        .expect_err("slow provider must exceed zero duration");
    assert!(error.to_string().contains("provider duration budget"));
}

/// Repeatedly requests a tool so the runtime, rather than the provider, hits
/// the configured budget.
struct RepeatedToolProvider;

impl ModelProvider for RepeatedToolProvider {
    /// Returns the same deterministic tool request on every step.
    ///
    /// # Parameters
    /// - `_messages`: Current runtime messages.
    ///
    /// # Returns
    /// A deterministic tool decision.
    ///
    /// # Errors
    /// This fixture never returns an error.
    fn decide(&mut self, _messages: &[AgentMessage]) -> Result<ModelDecision, String> {
        Ok(ModelDecision::Tool(ToolInput {
            entity: EntityId::from_bytes([4; 16]),
            snapshot: None,
            detail: ToolDetail::Compact,
        }))
    }
}

/// Accepts the repeated test tool without touching application state.
struct NoopTool;

/// Sleeps briefly to exercise provider-duration accounting.
struct SlowProvider;

impl ModelProvider for SlowProvider {
    /// Returns a final response after a measurable delay.
    ///
    /// # Parameters
    /// - `_messages`: Current runtime messages.
    ///
    /// # Returns
    /// A delayed final decision.
    ///
    /// # Errors
    /// This fixture never returns an error.
    fn decide(&mut self, _messages: &[AgentMessage]) -> Result<ModelDecision, String> {
        std::thread::sleep(std::time::Duration::from_millis(1));
        Ok(ModelDecision::Final("slow".to_owned()))
    }
}

impl Tool for NoopTool {
    /// Returns the stable fixture tool identifier.
    ///
    /// # Returns
    /// The fixture tool name.
    fn name(&self) -> &'static str {
        "noop"
    }

    /// Returns one bounded fixture result.
    ///
    /// # Parameters
    /// - `_context`: Runtime application context.
    /// - `input`: Tool input to echo.
    ///
    /// # Returns
    /// A bounded fixture result.
    ///
    /// # Errors
    /// This fixture never returns an error.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Ok(ToolResult {
            entity: input.entity,
            object_type: None,
            revision: None,
            snapshot: input.snapshot,
            summary: Some("noop".to_owned()),
            handles: Vec::new(),
        })
    }
}
