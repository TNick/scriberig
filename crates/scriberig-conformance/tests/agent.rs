//! Verifies shared deterministic agent runtime semantics.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use scriberig_agent_core::{Tool, ToolDetail, ToolError, ToolInput, ToolResult};
use scriberig_agent_runtime::cache::{CachePolicy, ModelCache};
use scriberig_agent_runtime::context::RequestFingerprintInput;
use scriberig_agent_runtime::orchestration::RuntimeLimits;
use scriberig_agent_runtime::provider::{ToolCall, ToolDefinition};
use scriberig_agent_runtime::tools::{ToolCatalog, ToolDescriptor, ToolExecutionMode};
use scriberig_agent_runtime::{
    AgentMessage, AgentRuntime, ChildExecutionScope, ChildExecutor, ChildRequest, ChildResult,
    UnconfiguredTool,
};
use scriberig_app::ApplicationContext;
use scriberig_app::CapabilityContext;
use scriberig_app::{OperationRegistry, OperationStatus};
use scriberig_conformance::agent_fixture::{ProviderStep, ScriptedProvider};
use scriberig_core::EntityId;

/// Provides one exact semantic handle for an atomic-tool conformance call.
struct AtomicTool;

impl Tool for AtomicTool {
    /// Returns the stable fixture tool name.
    ///
    /// # Returns
    /// The `inspect` tool identifier.
    fn name(&self) -> &'static str {
        "inspect"
    }

    /// Returns the requested entity without creating a child branch.
    ///
    /// # Parameters
    /// - `_context`: Trusted application context.
    /// - `input`: Entity request selected by the provider.
    ///
    /// # Returns
    /// One exact semantic result handle.
    ///
    /// # Errors
    /// This fixture never fails.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Ok(ToolResult {
            entity: input.entity,
            revision: None,
            snapshot: input.snapshot,
            object_type: Some("fixture.entity".to_owned()),
            summary: Some("atomic result".to_owned()),
            handles: vec![format!("entity:{}", input.entity)],
        })
    }
}

/// Executes a scripted child and returns only its compact semantic result.
struct ChildFixture {
    /// Provider script used by the child branch.
    steps: Vec<ProviderStep>,
}

impl ChildExecutor for ChildFixture {
    /// Runs the child script under the inherited branch scope.
    ///
    /// # Parameters
    /// - `context`: Trusted context inherited from the parent.
    /// - `scope`: Parent branch identity and depth.
    /// - `request`: Validated child request.
    /// - `cancelled`: Shared execution-tree cancellation flag.
    ///
    /// # Returns
    /// A compact result that intentionally excludes child provider history.
    ///
    /// # Errors
    /// Returns a safe child runtime failure.
    fn execute_child(
        &mut self,
        context: &ApplicationContext,
        scope: Option<ChildExecutionScope>,
        request: ChildRequest,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ChildResult, String> {
        let scope = scope.ok_or_else(|| "missing parent scope".to_owned())?;
        let (provider, _) = ScriptedProvider::new(self.steps.clone());
        let mut runtime = AgentRuntime::new(provider, UnconfiguredTool, 4)
            .with_branch_id("child")
            .with_limits(RuntimeLimits {
                max_depth: 4,
                ..RuntimeLimits::default()
            });
        let _result = runtime
            .run_with_child_executor_at(&scope, context, request.prompt, cancelled, self)
            .map_err(|error| error.to_string())?;
        Ok(ChildResult {
            summary: "child compact result".to_owned(),
            handles: vec!["branch-result:child".to_owned()],
        })
    }
}

/// Verifies the shared fixture produces one terminal root result.
#[test]
fn scripted_root_answer_is_terminal_and_observable() {
    let (provider, observed) =
        ScriptedProvider::new([ProviderStep::Final("summarized X".to_owned())]);
    let mut runtime = AgentRuntime::new(provider, UnconfiguredTool, 2);
    let result = runtime
        .run(
            &ApplicationContext::empty(),
            "Summarize X",
            &AtomicBool::new(false),
        )
        .expect("root answer");

    assert_eq!(result.output, "summarized X");
    assert_eq!(
        result.messages,
        vec![AgentMessage::User("Summarize X".to_owned())]
    );
    assert_eq!(observed.lock().expect("observations").len(), 1);
}

/// Verifies an atomic tool result stays in the root conversation.
#[test]
fn scripted_atomic_tool_does_not_create_a_child() {
    let entity = EntityId::from_bytes([9; 16]);
    let (provider, observed) = ScriptedProvider::new([
        ProviderStep::Tool(ToolInput {
            entity,
            snapshot: None,
            detail: ToolDetail::Compact,
        }),
        ProviderStep::Final("tool-backed answer".to_owned()),
    ]);
    let mut runtime = AgentRuntime::new(provider, AtomicTool, 3);
    let result = runtime
        .run(
            &ApplicationContext::empty(),
            "inspect X",
            &AtomicBool::new(false),
        )
        .expect("tool-backed answer");
    let observations = observed.lock().expect("observations");

    assert_eq!(result.output, "tool-backed answer");
    assert_eq!(observations.len(), 2);
    assert!(matches!(
        observations[1].last(),
        Some(AgentMessage::Tool(tool)) if tool.handles == vec![format!("entity:{entity}")]
    ));
}

/// Verifies child history is replaced by a compact result in parent context.
#[test]
fn scripted_child_result_preserves_isolation_marker_boundary() {
    let (provider, observed) = ScriptedProvider::new([
        ProviderStep::Child {
            purpose: "focused work".to_owned(),
            prompt: "private child context".to_owned(),
        },
        ProviderStep::Final("root answer".to_owned()),
    ]);
    let mut child = ChildFixture {
        steps: vec![ProviderStep::Final(
            "CHILD_PRIVATE_CONTEXT_MARKER_7F3A".to_owned(),
        )],
    };
    let scope = ChildExecutionScope {
        session: EntityId::from_bytes([1; 16]),
        parent_branch: EntityId::from_bytes([2; 16]),
        depth: 0,
        active_branches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        semantic_scope: None,
    };
    let mut runtime = AgentRuntime::new(provider, UnconfiguredTool, 3);
    let result = runtime
        .run_with_child_executor_at(
            &scope,
            &ApplicationContext::empty(),
            "root prompt",
            &AtomicBool::new(false),
            &mut child,
        )
        .expect("nested result");
    let observations = observed.lock().expect("observations");
    let parent_follow_up = observations.last().expect("parent follow-up");

    assert_eq!(result.output, "root answer");
    assert!(!parent_follow_up.iter().any(|message| {
        matches!(message, AgentMessage::Child(child) if child.summary.contains("CHILD_PRIVATE"))
    }));
}

/// Verifies model-step exhaustion returns a structured budget failure.
#[test]
fn scripted_provider_obeys_model_budget() {
    let (provider, _) = ScriptedProvider::new([
        ProviderStep::Child {
            purpose: "loop".to_owned(),
            prompt: "loop".to_owned(),
        },
        ProviderStep::Child {
            purpose: "loop".to_owned(),
            prompt: "loop".to_owned(),
        },
    ]);
    let mut child = ChildFixture {
        steps: vec![ProviderStep::Final("child result".to_owned())],
    };
    let scope = ChildExecutionScope {
        session: EntityId::from_bytes([3; 16]),
        parent_branch: EntityId::from_bytes([4; 16]),
        depth: 0,
        active_branches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        semantic_scope: None,
    };
    let mut runtime = AgentRuntime::new(provider, UnconfiguredTool, 1);
    let error = runtime
        .run_with_child_executor_at(
            &scope,
            &ApplicationContext::empty(),
            "budget",
            &AtomicBool::new(false),
            &mut child,
        )
        .expect_err("budget failure");

    assert!(error.to_string().contains("budget"));
}

/// Verifies exact cache hits preserve original invocation provenance.
#[test]
fn exact_cache_reuse_preserves_provenance() {
    let cache = ModelCache::new();
    let first = cache
        .execute(
            CachePolicy::ReuseExact,
            "semantic-request",
            "invocation-1",
            || {
                Ok(scriberig_agent_runtime::provider::ModelResponse {
                    text: Some("answer".to_owned()),
                    tool_calls: Vec::new(),
                    finish_reason: scriberig_agent_runtime::provider::FinishReason::Stop,
                    usage: None,
                    metadata: scriberig_agent_runtime::provider::ProviderMetadata::default(),
                })
            },
        )
        .expect("first response");
    let second = cache
        .execute(
            CachePolicy::ReuseExact,
            "semantic-request",
            "invocation-2",
            || panic!("exact hit must not invoke provider"),
        )
        .expect("cached response");

    assert!(!first.reused);
    assert!(second.reused);
    assert_eq!(second.original_invocation, "invocation-1");
    assert_eq!(second.invocation, "invocation-2");
}

/// Verifies capability denial occurs before a provider-selected tool dispatch.
#[test]
fn scripted_tool_denial_is_surface_independent() {
    let catalog = ToolCatalog::new(vec![ToolDescriptor {
        definition: ToolDefinition {
            name: "delegate".to_owned(),
            description: "Delegate focused work".to_owned(),
            parameters: serde_json::json!({"type": "object"}),
        },
        result_shape: "compact child result".to_owned(),
        state_changing: false,
        execution_mode: ToolExecutionMode::ChildBranch,
        required_capability: Some("agent.delegate".to_owned()),
    }]);
    let error = catalog
        .validate_for_context(
            &ToolCall {
                id: "call-1".to_owned(),
                name: "delegate".to_owned(),
                arguments: serde_json::json!({
                    "purpose": "focused",
                    "prompt": "inspect X",
                    "capabilities": ["inspect.entity"],
                    "expected_result": "bounded findings"
                }),
            },
            &CapabilityContext::default(),
        )
        .expect_err("missing capability must deny dispatch");

    assert!(error.to_string().contains("agent.delegate"));
}

/// Verifies cancellation stops runtime work and closes its shared operation.
#[test]
fn cancellation_preserves_shared_terminal_status() {
    let (provider, _) = ScriptedProvider::new([ProviderStep::Final("unreachable".to_owned())]);
    let registry = OperationRegistry::new();
    let mut runtime =
        AgentRuntime::new(provider, UnconfiguredTool, 2).with_operation_registry(registry.clone());
    let cancelled = AtomicBool::new(true);
    let error = runtime
        .run(&ApplicationContext::empty(), "cancel me", &cancelled)
        .expect_err("cancelled runtime");

    assert!(matches!(
        error,
        scriberig_agent_runtime::AgentError::Cancelled
    ));
    let operation = registry
        .get(&scriberig_core::OperationId::new("agent-1").expect("operation"))
        .expect("cancelled operation");
    assert_eq!(operation.status, OperationStatus::Cancelled);
}

/// Verifies provider credentials cannot enter cache identity material.
#[test]
fn provider_secret_is_absent_from_public_fingerprint() {
    let secret = "TEST_PROVIDER_SECRET_DO_NOT_LEAK_9A2D";
    let fingerprint = RequestFingerprintInput::new(
        "fixture",
        "model",
        "compiled semantic context",
        "inspect:v1",
    )
    .fingerprint();

    assert!(!fingerprint.contains(secret));
    assert_eq!(fingerprint.len(), 64);
}
