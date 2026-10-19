//! Verifies cancellation that arrives while the provider is working.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use scriberig_agent_core::{Tool, ToolDetail, ToolError, ToolInput, ToolResult};
use scriberig_agent_runtime::{AgentMessage, AgentRuntime, ModelDecision, ModelProvider};
use scriberig_app::ApplicationContext;
use scriberig_core::EntityId;

/// Verifies cancellation received during provider work prevents its tool call.
#[test]
fn runtime_does_not_execute_tool_after_provider_returns_to_cancelled_branch() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut runtime = AgentRuntime::new(
        CancellingProvider(Arc::clone(&cancelled)),
        CountingTool(Arc::clone(&calls)),
        1,
    );

    let error = runtime
        .run(
            &ApplicationContext::empty(),
            "cancel during provider",
            &cancelled,
        )
        .expect_err("provider response must not continue a cancelled branch");

    assert_eq!(error, scriberig_agent_runtime::AgentError::Cancelled);
    assert_eq!(calls.load(Ordering::Acquire), 0);
}

/// Sets cancellation while returning an otherwise valid tool request.
struct CancellingProvider(Arc<AtomicBool>);

impl ModelProvider for CancellingProvider {
    /// Cancels the branch during provider work and returns a tool request.
    ///
    /// - `_messages`: Current conversation.
    ///
    /// A tool request that must be discarded after cancellation.
    ///
    /// This fixture never returns an error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn decide(&mut self, _messages: &[AgentMessage]) -> Result<ModelDecision, String> {
        self.0.store(true, Ordering::Release);
        Ok(ModelDecision::Tool(ToolInput {
            entity: EntityId::from_bytes([4; 16]),
            snapshot: None,
            detail: ToolDetail::Compact,
        }))
    }
}

/// Counts tool invocations for the cancellation race test.
struct CountingTool(Arc<AtomicUsize>);

impl Tool for CountingTool {
    /// Returns the fixture tool identifier.
    ///
    /// The stable name `counting`.
    /// # Returns
    /// The operation result.
    fn name(&self) -> &'static str {
        "counting"
    }

    /// Counts one invocation and returns its input unchanged.
    ///
    /// - `_context`: Runtime request context.
    /// - `input`: Tool request to accept.
    ///
    /// A bounded successful tool result.
    ///
    /// This fixture never returns an error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        self.0.fetch_add(1, Ordering::AcqRel);
        Ok(ToolResult {
            entity: input.entity,
            object_type: None,
            revision: None,
            snapshot: input.snapshot,
            summary: Some("counted".to_owned()),
            handles: Vec::new(),
        })
    }
}
