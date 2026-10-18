//! Verifies execution-tree-wide active branch budgeting.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize};

use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_agent_runtime::{
    AgentRuntime, ChildExecutor, ChildRequest, ChildResult, EchoProvider,
};
use scriberig_app::ApplicationContext;
use scriberig_core::EntityId;

/// Verifies a branch cannot start when all shared active slots are occupied.
#[test]
fn active_branch_budget_is_shared_by_scope() {
    let active = Arc::new(AtomicUsize::new(1));
    let scope = scriberig_agent_runtime::ChildExecutionScope {
        session: EntityId::from_bytes([6; 16]),
        parent_branch: EntityId::from_bytes([7; 16]),
        depth: 0,
        active_branches: active,
        semantic_scope: None,
    };
    let mut runtime = AgentRuntime::new(EchoProvider, NoopTool, 1).with_limits(
        scriberig_agent_runtime::orchestration::RuntimeLimits {
            max_active_branches: 1,
            ..Default::default()
        },
    );
    let error = runtime
        .run_with_child_executor_at(
            &scope,
            &ApplicationContext::empty(),
            "active",
            &AtomicBool::new(false),
            &mut NoopChildExecutor,
        )
        .expect_err("active branch budget must be enforced");
    assert!(error.to_string().contains("active branch budget"));
}

/// Accepts a tool without touching application state.
struct NoopTool;

impl Tool for NoopTool {
    /// Returns the fixture tool identifier.
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

/// Provides a child executor that is never reached by the budget test.
struct NoopChildExecutor;

impl ChildExecutor for NoopChildExecutor {
    /// Returns a bounded fixture child result.
    ///
    /// # Parameters
    /// - `_context`: Runtime application context.
    /// - `_scope`: Optional durable parent scope.
    /// - `_request`: Validated child request.
    /// - `_cancelled`: Shared cancellation flag.
    ///
    /// # Returns
    /// A bounded fixture child result.
    ///
    /// # Errors
    /// This fixture never returns an error.
    fn execute_child(
        &mut self,
        _context: &ApplicationContext,
        _scope: Option<scriberig_agent_runtime::ChildExecutionScope>,
        _request: ChildRequest,
        _cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ChildResult, String> {
        Ok(ChildResult {
            summary: "unused".to_owned(),
            handles: Vec::new(),
            branch: None,
        })
    }
}
