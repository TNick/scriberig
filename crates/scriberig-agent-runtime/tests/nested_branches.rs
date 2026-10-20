//! Verifies recursive branch topology and compact parent projections.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use scriberig_agent_runtime::{
    AgentMessage, AgentRuntime, ChildExecutionScope, ChildExecutor, ChildRequest, ChildResult,
    ModelDecision, ModelProvider, UnconfiguredTool,
};
use scriberig_app::ApplicationContext;
use scriberig_core::EntityId;

/// Produces a bounded delegation tree and then a final response.
struct RecursiveProvider {
    /// Current semantic branch depth.
    depth: usize,
    /// Last depth allowed to delegate.
    delegation_limit: usize,
}

/// Completes one model step and records how many requests the child made.
struct CountingFinalProvider {
    /// Number of child provider invocations.
    calls: Arc<std::sync::atomic::AtomicUsize>,
    /// Final child output returned by the provider.
    output: String,
}

impl ModelProvider for CountingFinalProvider {
    /// Returns the configured final output and increments the invocation count.
    ///
    /// - `_messages`: Current child conversation, unused by this fixture.
    ///
    /// A final response for the child branch.
    ///
    /// This deterministic provider does not fail.
    /// # Returns
    /// The final model decision.
    /// # Errors
    /// Never returns an error.
    fn decide(&mut self, _messages: &[AgentMessage]) -> Result<ModelDecision, String> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        Ok(ModelDecision::Final(self.output.clone()))
    }
}

/// Delegates once, then finishes after the compact child result arrives.
struct OneChildParentProvider;

impl ModelProvider for OneChildParentProvider {
    /// Requests one child and checks the follow-up contains only its compact result.
    ///
    /// - `messages`: Current parent conversation.
    ///
    /// A child request on the first call and a final response afterward.
    ///
    /// This deterministic provider does not fail.
    /// # Returns
    /// The next parent model decision.
    /// # Errors
    /// Never returns an error.
    fn decide(&mut self, messages: &[AgentMessage]) -> Result<ModelDecision, String> {
        if messages
            .iter()
            .any(|message| matches!(message, AgentMessage::Child(_)))
        {
            assert!(!messages.iter().any(|message| {
                matches!(message, AgentMessage::User(text) if text.contains("CHILD_PRIVATE_TRANSCRIPT"))
            }));
            Ok(ModelDecision::Final("parent answer".to_owned()))
        } else {
            Ok(ModelDecision::Child(ChildRequest {
                purpose: "focused work".to_owned(),
                prompt: "CHILD_PRIVATE_TRANSCRIPT".to_owned(),
                capabilities: Vec::new(),
                expected_result: "one compact summary".to_owned(),
            }))
        }
    }
}

/// Runs a one-step child and projects its final output and handles to the parent.
struct OneStepChildExecutor {
    /// Number of calls made by the child provider.
    calls: Arc<std::sync::atomic::AtomicUsize>,
}

impl ChildExecutor for OneStepChildExecutor {
    /// Executes the child once and returns its final output as compact summary.
    ///
    /// - `context`: Trusted context inherited by the child.
    /// - `scope`: Durable parent branch scope.
    /// - `request`: Validated purpose and child prompt.
    /// - `cancelled`: Shared execution cancellation flag.
    ///
    /// The child final output, stable handle, and branch identity.
    ///
    /// Returns an error when the parent scope is missing or child execution fails.
    /// # Returns
    /// The compact result produced for the parent branch.
    /// # Errors
    /// Returns an error when child execution cannot complete.
    fn execute_child(
        &mut self,
        context: &ApplicationContext,
        scope: Option<ChildExecutionScope>,
        request: ChildRequest,
        cancelled: &AtomicBool,
    ) -> Result<ChildResult, String> {
        let scope = scope.ok_or_else(|| "missing parent scope".to_owned())?;
        let mut runtime = AgentRuntime::new(
            CountingFinalProvider {
                calls: Arc::clone(&self.calls),
                output: "final child output".to_owned(),
            },
            UnconfiguredTool,
            1,
        );
        let result = runtime
            .run_with_child_executor_at(&scope, context, request.prompt, cancelled, self)
            .map_err(|error| error.to_string())?;
        Ok(ChildResult {
            summary: result.output,
            handles: vec!["entity:child-result".to_owned()],
            branch: Some(EntityId::from_bytes([7; 16])),
        })
    }
}

impl ModelProvider for RecursiveProvider {
    /// Delegates until the configured depth and then completes.
    ///
    /// - `messages`: Current branch conversation.
    ///
    /// A child request or a compact final response.
    ///
    /// This deterministic provider never fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn decide(&mut self, messages: &[AgentMessage]) -> Result<ModelDecision, String> {
        let completed_children = messages
            .iter()
            .filter(|message| matches!(message, AgentMessage::Child(_)))
            .count();
        if self.depth == 0 && completed_children < 2 {
            let purpose = if completed_children == 0 {
                "architecture"
            } else {
                "implementation"
            };
            return Ok(ModelDecision::Child(ChildRequest {
                purpose: purpose.to_owned(),
                prompt: format!("perform {purpose} work"),
                capabilities: vec!["inspect.entity".to_owned()],
                expected_result: "compact result".to_owned(),
            }));
        }
        if messages.len() == 1 && self.depth < self.delegation_limit {
            return Ok(ModelDecision::Child(ChildRequest {
                purpose: format!("depth-{} work", self.depth + 1),
                prompt: format!("perform depth-{} work", self.depth + 1),
                capabilities: vec!["inspect.entity".to_owned()],
                expected_result: "compact result".to_owned(),
            }));
        }
        if let Some(AgentMessage::Child(result)) = messages.last() {
            assert!(!result.summary.contains("private-parent-history"));
        }
        Ok(ModelDecision::Final(format!(
            "completed-depth-{}",
            self.depth
        )))
    }
}

/// Recursively invokes the same runtime with an explicit child scope.
struct RecursiveExecutor {
    /// Observed child depths.
    depths: Arc<Mutex<Vec<usize>>>,
}

impl ChildExecutor for RecursiveExecutor {
    /// Executes one nested branch and projects only its compact result.
    ///
    /// - `context`: Trusted context inherited by the child.
    /// - `scope`: Parent branch scope.
    /// - `request`: Validated child request.
    /// - `cancelled`: Shared cancellation flag.
    ///
    /// A compact nested result.
    ///
    /// Returns a nested runtime failure as safe text.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute_child(
        &mut self,
        context: &ApplicationContext,
        scope: Option<ChildExecutionScope>,
        request: ChildRequest,
        cancelled: &AtomicBool,
    ) -> Result<ChildResult, String> {
        let parent = scope.ok_or_else(|| "missing parent scope".to_owned())?;
        let child = ChildExecutionScope {
            session: parent.session,
            parent_branch: EntityId::from_bytes(
                [u8::try_from(parent.depth).expect("test depth fits") + 1; 16],
            ),
            depth: parent.depth + 1,
            active_branches: Arc::clone(&parent.active_branches),
            semantic_scope: parent.semantic_scope.clone(),
        };
        self.depths
            .lock()
            .map_err(|_| "depth lock failed".to_owned())?
            .push(child.depth);
        let mut nested = Self {
            depths: Arc::clone(&self.depths),
        };
        let mut runtime = AgentRuntime::new(
            RecursiveProvider {
                depth: child.depth,
                delegation_limit: 2,
            },
            UnconfiguredTool,
            4,
        );
        let result = runtime
            .run_with_child_executor_at(&child, context, request.prompt, cancelled, &mut nested)
            .map_err(|error| error.to_string())?;
        Ok(ChildResult {
            summary: result.output,
            handles: vec![format!("child-depth-{}", child.depth)],
            branch: None,
        })
    }
}

/// Verifies nested scopes, shared branch budgeting, and compact isolation.
#[test]
fn recursive_children_preserve_depth_and_hide_child_history() {
    let depths = Arc::new(Mutex::new(Vec::new()));
    let scope = ChildExecutionScope {
        session: EntityId::from_bytes([1; 16]),
        parent_branch: EntityId::from_bytes([2; 16]),
        depth: 0,
        active_branches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        semantic_scope: None,
    };
    let mut executor = RecursiveExecutor {
        depths: Arc::clone(&depths),
    };
    let mut runtime = AgentRuntime::new(
        RecursiveProvider {
            depth: 0,
            delegation_limit: 1,
        },
        UnconfiguredTool,
        4,
    );
    let result = runtime
        .run_with_child_executor_at(
            &scope,
            &ApplicationContext::empty(),
            "root prompt",
            &AtomicBool::new(false),
            &mut executor,
        )
        .expect("recursive execution");

    assert_eq!(result.output, "completed-depth-0");
    assert_eq!(result.messages.len(), 3);
    assert!(matches!(result.messages[1], AgentMessage::Child(_)));
    assert!(matches!(result.messages[2], AgentMessage::Child(_)));
    assert_eq!(*depths.lock().expect("depths"), vec![1, 2, 1, 2]);
    assert!(result.messages.iter().all(|message| match message {
        AgentMessage::Child(child) => !child.summary.contains("private-worker-transcript"),
        _ => true,
    }));
    assert_eq!(
        scope
            .active_branches
            .load(std::sync::atomic::Ordering::Acquire),
        0
    );
}

/// Verifies final child output stays separate from handles without a summarizer call.
#[test]
fn child_final_output_is_compact_result_without_extra_model_call() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let scope = ChildExecutionScope {
        session: EntityId::from_bytes([1; 16]),
        parent_branch: EntityId::from_bytes([2; 16]),
        depth: 0,
        active_branches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        semantic_scope: None,
    };
    let mut executor = OneStepChildExecutor {
        calls: Arc::clone(&calls),
    };
    let mut runtime = AgentRuntime::new(OneChildParentProvider, UnconfiguredTool, 2);
    let result = runtime
        .run_with_child_executor_at(
            &scope,
            &ApplicationContext::empty(),
            "parent task",
            &AtomicBool::new(false),
            &mut executor,
        )
        .expect("parent completes after child");

    assert_eq!(result.output, "parent answer");
    assert_eq!(calls.load(std::sync::atomic::Ordering::Acquire), 1);
    assert!(matches!(
        result.messages.get(1),
        Some(AgentMessage::Child(child))
            if child.summary == "final child output"
                && child.handles == vec!["entity:child-result"]
                && child.branch == Some(EntityId::from_bytes([7; 16]))
    ));
}
