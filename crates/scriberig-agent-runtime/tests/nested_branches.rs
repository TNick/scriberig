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

impl ModelProvider for RecursiveProvider {
    /// Delegates until the configured depth and then completes.
    ///
    /// # Parameters
    /// - `messages`: Current branch conversation.
    ///
    /// # Returns
    /// A child request or a compact final response.
    ///
    /// # Errors
    /// This deterministic provider never fails.
    fn decide(&mut self, messages: &[AgentMessage]) -> Result<ModelDecision, String> {
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
    /// # Parameters
    /// - `context`: Trusted context inherited by the child.
    /// - `scope`: Parent branch scope.
    /// - `request`: Validated child request.
    /// - `cancelled`: Shared cancellation flag.
    ///
    /// # Returns
    /// A compact nested result.
    ///
    /// # Errors
    /// Returns a nested runtime failure as safe text.
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
    assert_eq!(result.messages.len(), 2);
    assert!(matches!(result.messages[1], AgentMessage::Child(_)));
    assert_eq!(*depths.lock().expect("depths"), vec![1, 2]);
    assert_eq!(
        scope
            .active_branches
            .load(std::sync::atomic::Ordering::Acquire),
        0
    );
}
