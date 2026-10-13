//! Implements tool, budget, lifecycle, and event helpers for the runtime.

use std::sync::atomic::AtomicBool;

use scriberig_agent_core::Tool;
use scriberig_app::{ApplicationContext, OperationId, OperationStatus};

use crate::orchestration::{BranchPhase, BranchRuntimeState, RuntimeEvent};
use crate::{
    AgentError, ChildExecutionScope, ChildExecutor, ChildRequest, ChildResult, ModelProvider,
};

use super::AgentRuntime;

impl<P, T> AgentRuntime<P, T>
where
    P: ModelProvider,
    T: Tool,
{
    /// Executes one atomic tool after reserving its runtime budget.
    ///
    /// # Parameters
    /// - `context`: Trusted application context.
    /// - `super`: Runtime owner.
    /// - `input`: Validated atomic tool input.
    /// - `operation`: Optional lifecycle operation.
    /// - `state`: Mutable branch counters.
    ///
    /// # Returns
    /// The bounded application tool result.
    ///
    /// # Errors
    /// Returns a tool or budget failure.
    pub(super) fn run_atomic_tool(
        &self,
        context: &ApplicationContext,
        input: scriberig_agent_core::ToolInput,
        operation: Option<&OperationId>,
        state: &mut BranchRuntimeState,
    ) -> Result<scriberig_agent_core::ToolResult, AgentError> {
        self.reserve_tool_call(operation, state)?;
        self.emit(BranchPhase::ExecutingTools);
        self.emit_event(RuntimeEvent::tool_started(&self.branch_id, "inspect"));
        let result = self.tool.invoke(context, input).map_err(|error| {
            self.emit_event(RuntimeEvent::tool_completed(
                &self.branch_id,
                "inspect",
                false,
            ));
            self.finish_operation(operation, OperationStatus::Failed);
            AgentError::Tool(error)
        })?;
        self.emit_event(RuntimeEvent::tool_completed(
            &self.branch_id,
            "inspect",
            true,
        ));
        Ok(result)
    }

    /// Executes one named application control tool.
    ///
    /// # Parameters
    /// - `self`: Runtime owning the tool boundary.
    /// - `super`: Runtime helper module containing the dispatch implementation.
    /// - `context`: Trusted application request context.
    /// - `name`: Registered control-tool identifier.
    /// - `arguments`: Provider arguments validated by the catalog.
    /// - `cancelled`: Shared cancellation flag for the active branch.
    /// - `operation`: Optional lifecycle operation.
    /// - `state`: Mutable branch counters.
    ///
    /// # Returns
    /// The bounded application control result.
    ///
    /// # Errors
    /// Returns a tool or budget failure.
    pub(super) fn run_named_tool(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
        cancelled: &AtomicBool,
        operation: Option<&OperationId>,
        state: &mut BranchRuntimeState,
    ) -> Result<scriberig_agent_core::ToolResult, AgentError> {
        self.reserve_tool_call(operation, state)?;
        self.emit(BranchPhase::ExecutingTools);
        self.emit_event(RuntimeEvent::tool_started(&self.branch_id, name));
        let result = self
            .tool
            .invoke_named_cancellable(context, name, arguments, cancelled)
            .map_err(|error| {
                self.emit_event(RuntimeEvent::tool_completed(&self.branch_id, name, false));
                if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                    self.emit(BranchPhase::Cancelled);
                    self.emit_event(RuntimeEvent::branch_execution_cancelled(&self.branch_id));
                    self.finish_operation(operation, OperationStatus::Cancelled);
                    AgentError::Cancelled
                } else {
                    self.finish_operation(operation, OperationStatus::Failed);
                    AgentError::Tool(error)
                }
            })?;
        self.emit_event(RuntimeEvent::tool_completed(&self.branch_id, name, true));
        Ok(result)
    }

    /// Executes one delegated child after reserving its branch budgets.
    ///
    /// # Parameters
    /// - `context`: Trusted application context.
    /// - `super`: Runtime owner.
    /// - `scope`: Durable parent branch scope.
    /// - `request`: Validated child request.
    /// - `cancelled`: Shared cancellation flag.
    /// - `operation`: Optional lifecycle operation.
    /// - `super`: Runtime owner.
    /// - `state`: Mutable branch counters.
    /// - `children`: Optional recursive executor.
    ///
    /// # Returns
    /// The compact child result.
    ///
    /// # Errors
    /// Returns a child, provider, or budget failure.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn run_child_tool(
        &mut self,
        context: &ApplicationContext,
        scope: Option<ChildExecutionScope>,
        request: ChildRequest,
        cancelled: &AtomicBool,
        operation: Option<&OperationId>,
        state: &mut BranchRuntimeState,
        children: &mut Option<&mut dyn ChildExecutor>,
    ) -> Result<ChildResult, AgentError> {
        self.reserve_tool_call(operation, state)?;
        if state.child_branches >= self.limits.max_child_branches {
            self.fail_budget(operation, "child branch", self.limits.max_child_branches)?;
        }
        state.child_branches += 1;
        self.emit(BranchPhase::ExecutingTools);
        self.emit_event(RuntimeEvent::tool_started(&self.branch_id, "delegate"));
        self.emit_event(RuntimeEvent::child_branch_started(&self.branch_id));
        let Some(executor) = children.as_deref_mut() else {
            self.finish_operation(operation, OperationStatus::Failed);
            return Err(AgentError::Provider(
                "child delegation is not configured".to_owned(),
            ));
        };
        let result = executor
            .execute_child(context, scope, request, cancelled)
            .map_err(|error| {
                self.emit_event(RuntimeEvent::tool_completed(
                    &self.branch_id,
                    "delegate",
                    false,
                ));
                self.emit_event(RuntimeEvent::child_branch_completed(&self.branch_id, false));
                self.finish_operation(operation, OperationStatus::Failed);
                AgentError::Provider(format!("child branch: {error}"))
            })?;
        self.emit_event(RuntimeEvent::tool_completed(
            &self.branch_id,
            "delegate",
            true,
        ));
        self.emit_event(RuntimeEvent::child_branch_completed(&self.branch_id, true));
        Ok(result)
    }

    /// Reserves one model step or emits a terminal budget failure.
    ///
    /// # Parameters
    /// - `operation`: Optional lifecycle operation.
    /// - `super`: Runtime owner.
    /// - `state`: Mutable branch counters.
    ///
    /// # Returns
    /// Unit after reserving one model step.
    ///
    /// # Errors
    /// Returns a step-budget failure.
    pub(super) fn reserve_model_step(
        &self,
        operation: Option<&OperationId>,
        state: &mut BranchRuntimeState,
    ) -> Result<(), AgentError> {
        if state.model_steps >= self.limits.max_model_steps {
            self.fail_budget(operation, "model step", self.limits.max_model_steps)?;
        }
        state.model_steps += 1;
        Ok(())
    }

    /// Reserves one atomic or child tool call.
    ///
    /// # Parameters
    /// - `operation`: Optional lifecycle operation.
    /// - `super`: Runtime owner.
    /// - `state`: Mutable branch counters.
    ///
    /// # Returns
    /// Unit after reserving one tool call.
    ///
    /// # Errors
    /// Returns a tool-budget failure.
    pub(super) fn reserve_tool_call(
        &self,
        operation: Option<&OperationId>,
        state: &mut BranchRuntimeState,
    ) -> Result<(), AgentError> {
        if state.tool_calls >= self.limits.max_tool_calls {
            self.fail_budget(operation, "tool call", self.limits.max_tool_calls)?;
        }
        state.tool_calls += 1;
        Ok(())
    }

    /// Records a budget terminal phase and operation failure.
    ///
    /// # Parameters
    /// - `super`: Runtime owner.
    /// - `operation`: Optional lifecycle operation.
    /// - `kind`: Stable budget category.
    /// - `limit`: Configured maximum.
    ///
    /// # Returns
    /// Always returns the structured budget error.
    ///
    /// # Errors
    /// Always returns `AgentError::BudgetExceeded`.
    pub(super) fn fail_budget(
        &self,
        operation: Option<&OperationId>,
        kind: &'static str,
        limit: usize,
    ) -> Result<(), AgentError> {
        self.finish_operation(operation, OperationStatus::Failed);
        self.emit(BranchPhase::BudgetExceeded);
        self.emit_event(RuntimeEvent::branch_execution_failed(
            &self.branch_id,
            format!("{kind} budget"),
        ));
        Err(AgentError::BudgetExceeded { kind, limit })
    }

    /// Starts one tracked agent operation when a registry is configured.
    ///
    /// # Parameters
    /// - `super`: Runtime owner.
    /// # Returns
    /// An operation identity when tracking is enabled.
    ///
    /// # Errors
    /// Returns a provider error when lifecycle registration fails.
    pub(super) fn start_operation(&mut self) -> Result<Option<OperationId>, AgentError> {
        let Some(registry) = &self.registry else {
            return Ok(None);
        };
        let id = OperationId::new(format!("agent-{}", self.next_operation))
            .map_err(|error| AgentError::Provider(error.to_string()))?;
        self.next_operation += 1;
        registry
            .start(id.clone(), scriberig_app::OperationKind::Agent)
            .map_err(|error| AgentError::Provider(error.to_string()))?;
        registry
            .transition(&id, OperationStatus::Running)
            .map_err(|error| AgentError::Provider(error.to_string()))?;
        Ok(Some(id))
    }

    /// Publishes a terminal state for one tracked operation.
    ///
    /// # Parameters
    /// - `super`: Runtime owner.
    /// - `operation`: Optional operation identity.
    /// - `status`: Terminal status.
    pub(super) fn finish_operation(
        &self,
        operation: Option<&OperationId>,
        status: OperationStatus,
    ) {
        if let (Some(registry), Some(operation)) = (&self.registry, operation) {
            let _ = registry.transition(operation, status);
        }
    }

    /// Emits one safe phase event when a log is attached.
    ///
    /// # Parameters
    /// - `super`: Runtime owner.
    /// - `phase`: Runtime phase.
    pub(super) fn emit(&self, phase: BranchPhase) {
        if let Some(log) = &self.event_log {
            log.push(RuntimeEvent::phase_changed(&self.branch_id, phase));
        }
    }

    /// Emits one normalized event when a log is attached.
    ///
    /// # Parameters
    /// - `super`: Runtime owner.
    /// - `event`: Safe runtime event.
    pub(super) fn emit_event(&self, event: RuntimeEvent) {
        if let Some(log) = &self.event_log {
            log.push(event);
        }
    }

    /// Acquires one shared execution-tree branch slot.
    ///
    /// # Parameters
    /// - `super`: Runtime owner.
    /// - `scope`: Optional shared execution-tree scope.
    ///
    /// # Returns
    /// A guard that releases the slot when dropped.
    ///
    /// # Errors
    /// Returns an active-branch budget failure when no slot is available.
    pub(super) fn acquire_active_branch(
        &self,
        scope: Option<&ChildExecutionScope>,
    ) -> Result<super::ActiveBranchGuard, AgentError> {
        let active_count = scope.map_or_else(
            || std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            |value| std::sync::Arc::clone(&value.active_branches),
        );
        let active_before = active_count.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        if active_before >= self.limits.max_active_branches {
            active_count.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
            self.emit(BranchPhase::BudgetExceeded);
            self.emit_event(RuntimeEvent::branch_execution_failed(
                &self.branch_id,
                "active branch budget",
            ));
            return Err(AgentError::BudgetExceeded {
                kind: "active branch",
                limit: self.limits.max_active_branches,
            });
        }
        Ok(super::ActiveBranchGuard {
            count: active_count,
        })
    }
}
