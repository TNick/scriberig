//! Defines branch runtime state, limits, cancellation, and bounded events.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

/// Runtime phase separate from durable session branch status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BranchPhase {
    /// Branch identity has been allocated.
    Created,
    /// Provider-neutral context is being compiled.
    CompilingContext,
    /// Waiting to invoke the model.
    AwaitingModel,
    /// Interpreting a normalized model response.
    InterpretingModel,
    /// Executing validated atomic tools.
    ExecutingTools,
    /// Waiting for delegated child work.
    AwaitingChildren,
    /// Producing the durable branch result.
    Completing,
    /// Branch completed successfully.
    Completed,
    /// Branch failed.
    Failed,
    /// Branch was cancelled.
    Cancelled,
    /// Branch exceeded a configured budget.
    BudgetExceeded,
}

impl BranchPhase {
    /// Reports whether no further work may start in the phase.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::BudgetExceeded
        )
    }
}

/// Mutable counters and phase for one branch execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchRuntimeState {
    /// Stable runtime branch identity.
    pub branch: String,
    /// Current runtime phase.
    pub phase: BranchPhase,
    /// Number of model decisions already requested.
    pub model_steps: usize,
    /// Number of atomic/child tool calls already started.
    pub tool_calls: usize,
    /// Number of direct child branches already started.
    pub child_branches: usize,
    /// Current semantic branch depth.
    pub depth: usize,
}

impl BranchRuntimeState {
    /// Creates a new branch in the `Created` phase.
    ///
    /// # Parameters
    /// - `branch`: Stable branch identity.
    ///
    /// # Returns
    /// A zero-counter branch runtime state.
    #[must_use]
    pub fn new(branch: impl Into<String>) -> Self {
        Self {
            branch: branch.into(),
            phase: BranchPhase::Created,
            model_steps: 0,
            tool_calls: 0,
            child_branches: 0,
            depth: 0,
        }
    }

    /// Performs one explicitly validated runtime phase transition.
    ///
    /// # Parameters
    /// - `self`: Branch state being updated.
    /// - `next`: Desired next runtime phase.
    ///
    /// # Returns
    /// Unit after the transition is applied.
    ///
    /// # Errors
    /// Returns an error when the transition is not allowed.
    #[allow(clippy::unnested_or_patterns)]
    pub fn transition(&mut self, next: BranchPhase) -> Result<(), String> {
        if self.phase.is_terminal() {
            return Err(format!("terminal branch phase is {:?}", self.phase));
        }
        let valid = matches!(
            (self.phase, next),
            (BranchPhase::Created, BranchPhase::CompilingContext)
                | (BranchPhase::CompilingContext, BranchPhase::AwaitingModel)
                | (BranchPhase::AwaitingModel, BranchPhase::InterpretingModel)
                | (BranchPhase::InterpretingModel, BranchPhase::ExecutingTools)
                | (BranchPhase::InterpretingModel, BranchPhase::Completing)
                | (BranchPhase::ExecutingTools, BranchPhase::AwaitingModel)
                | (BranchPhase::ExecutingTools, BranchPhase::AwaitingChildren)
                | (BranchPhase::AwaitingChildren, BranchPhase::AwaitingModel)
                | (BranchPhase::Completing, BranchPhase::Completed)
                | (_, BranchPhase::Failed)
                | (_, BranchPhase::Cancelled)
                | (_, BranchPhase::BudgetExceeded)
        );
        if !valid {
            return Err(format!(
                "invalid branch transition {:?} -> {next:?}",
                self.phase
            ));
        }
        self.phase = next;
        Ok(())
    }
}

/// Maximum work permitted for one branch execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeLimits {
    /// Maximum provider decisions in one branch.
    pub max_model_steps: usize,
    /// Maximum tool calls in one branch.
    pub max_tool_calls: usize,
    /// Maximum direct child branches.
    pub max_child_branches: usize,
    /// Maximum nested branch depth.
    pub max_depth: usize,
    /// Maximum simultaneously active branches in one execution tree.
    pub max_active_branches: usize,
    /// Maximum elapsed time allowed for one provider call.
    pub max_provider_duration: std::time::Duration,
    /// Optional elapsed time allowed for one complete branch.
    pub max_total_duration: Option<std::time::Duration>,
}

impl Default for RuntimeLimits {
    /// Creates conservative bounded runtime limits.
    ///
    /// # Returns
    /// Default per-branch execution limits.
    fn default() -> Self {
        Self {
            max_model_steps: 32,
            max_tool_calls: 64,
            max_child_branches: 16,
            max_depth: 8,
            max_active_branches: 32,
            max_provider_duration: std::time::Duration::from_secs(300),
            max_total_duration: None,
        }
    }
}

impl RuntimeLimits {
    /// Checks counters before starting more branch work.
    ///
    /// # Parameters
    /// - `self`: Limits to enforce.
    /// - `state`: Current branch counters.
    ///
    /// # Returns
    /// Unit when all budgets remain available.
    ///
    /// # Errors
    /// Returns a bounded budget diagnostic when a limit is exhausted.
    pub fn check(&self, state: &BranchRuntimeState) -> Result<(), String> {
        if state.model_steps >= self.max_model_steps {
            return Err("model step budget exceeded".to_owned());
        }
        if state.tool_calls >= self.max_tool_calls {
            return Err("tool call budget exceeded".to_owned());
        }
        if state.child_branches >= self.max_child_branches {
            return Err("child branch budget exceeded".to_owned());
        }
        if state.depth > self.max_depth {
            return Err("branch depth budget exceeded".to_owned());
        }
        Ok(())
    }
}

/// Cancellation token that propagates from a parent to all children.
#[derive(Clone, Default)]
pub struct CancellationTree {
    state: Arc<CancellationState>,
}

/// Shared cancellation state for one branch tree.
#[derive(Default)]
struct CancellationState {
    cancelled: AtomicBool,
    children: Mutex<Vec<Weak<CancellationState>>>,
}

impl CancellationTree {
    /// Creates a clear root cancellation token.
    ///
    /// # Returns
    /// A non-cancelled root token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a child token linked to this parent.
    ///
    /// # Returns
    /// A child token cancelled by this parent.
    #[must_use]
    pub fn child(&self) -> Self {
        let child = Arc::new(CancellationState::default());
        if let Ok(mut children) = self.state.children.lock() {
            children.push(Arc::downgrade(&child));
        }
        Self { state: child }
    }

    /// Requests cancellation for this token and all currently linked children.
    pub fn cancel(&self) {
        self.state.cancelled.store(true, Ordering::Release);
        if let Ok(children) = self.state.children.lock() {
            for child in children.iter().filter_map(Weak::upgrade) {
                Self { state: child }.cancel();
            }
        }
    }

    /// Reports whether cancellation has reached this token.
    ///
    /// # Returns
    /// `true` when cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.state.cancelled.load(Ordering::Acquire)
    }
}

pub use crate::events::{RuntimeEvent, RuntimeEventLog};
