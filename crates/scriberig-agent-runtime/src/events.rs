//! Defines safe normalized runtime events and their bounded log.

use crate::orchestration::BranchPhase;

/// Safe bounded runtime observation.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum RuntimeEvent {
    /// A branch execution was allocated and began.
    BranchExecutionStarted { branch: String },
    /// A branch entered a new phase.
    PhaseChanged { branch: String, phase: BranchPhase },
    /// A provider request is about to be issued.
    ModelInvocationStarted { branch: String, step: usize },
    /// A bounded provider-neutral text fragment became available.
    ModelTextDelta { branch: String, text: String },
    /// Provider content explicitly intended for user display, never private
    /// reasoning or analysis.
    ReasoningSummary { branch: String, summary: String },
    /// An atomic or child tool was accepted for execution.
    ToolStarted { branch: String, tool: String },
    /// A tool execution returned or failed.
    ToolCompleted {
        branch: String,
        tool: String,
        success: bool,
    },
    /// A delegated child branch began.
    ChildBranchStarted { branch: String },
    /// A delegated child branch returned to its parent.
    ChildBranchCompleted { branch: String, success: bool },
    /// A compact branch result became available.
    BranchResultProduced { branch: String, summary: String },
    /// A branch completed successfully.
    BranchExecutionCompleted { branch: String },
    /// A branch failed with a bounded category.
    BranchExecutionFailed { branch: String, category: String },
    /// A branch was cancelled.
    BranchExecutionCancelled { branch: String },
}

/// Thread-safe bounded event collection for one runtime execution.
#[derive(Clone, Default)]
pub struct RuntimeEventLog {
    events: std::sync::Arc<std::sync::Mutex<Vec<RuntimeEvent>>>,
}

impl RuntimeEventLog {
    /// Creates an empty runtime event log.
    ///
    /// A shareable event log.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one bounded event.
    ///
    /// - `event`: Safe runtime event.
    pub fn push(&self, event: RuntimeEvent) {
        if let Ok(mut events) = self.events.lock() {
            events.push(event);
        }
    }

    /// Returns a snapshot of all events observed so far.
    ///
    /// Events in emission order.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn snapshot(&self) -> Vec<RuntimeEvent> {
        self.events
            .lock()
            .map_or_else(|_| Vec::new(), |events| events.clone())
    }
}

impl RuntimeEvent {
    /// Creates a safe phase transition event.
    ///
    /// - `branch`: Stable branch correlation identity.
    /// - `phase`: New runtime phase.
    ///
    /// A bounded runtime event.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn phase_changed(branch: impl Into<String>, phase: BranchPhase) -> Self {
        Self::PhaseChanged {
            branch: branch.into(),
            phase,
        }
    }

    /// Creates a branch-start event.
    ///
    /// - `branch`: Stable branch correlation identity.
    ///
    /// A bounded runtime event.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn branch_execution_started(branch: impl Into<String>) -> Self {
        Self::BranchExecutionStarted {
            branch: branch.into(),
        }
    }

    /// Creates a model invocation event.
    ///
    /// - `branch`: Stable branch correlation identity.
    /// - `step`: One-based model step number.
    ///
    /// A bounded runtime event.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn model_invocation_started(branch: impl Into<String>, step: usize) -> Self {
        Self::ModelInvocationStarted {
            branch: branch.into(),
            step,
        }
    }

    /// Creates a bounded model-text delta event.
    ///
    /// - `branch`: Stable branch correlation identity.
    /// - `text`: Incremental provider-neutral assistant text.
    ///
    /// A bounded runtime event.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn model_text_delta(branch: impl Into<String>, text: impl Into<String>) -> Self {
        let mut text = text.into();
        text.truncate(4096);
        Self::ModelTextDelta {
            branch: branch.into(),
            text,
        }
    }

    /// Creates a bounded tool-start event.
    ///
    /// - `branch`: Stable branch correlation identity.
    /// - `tool`: Trusted tool identifier.
    ///
    /// A bounded runtime event.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn tool_started(branch: impl Into<String>, tool: impl Into<String>) -> Self {
        Self::ToolStarted {
            branch: branch.into(),
            tool: tool.into(),
        }
    }

    /// Creates a bounded tool-completion event.
    ///
    /// - `branch`: Stable branch correlation identity.
    /// - `tool`: Trusted tool identifier.
    /// - `success`: Whether execution succeeded.
    ///
    /// A bounded runtime event.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn tool_completed(
        branch: impl Into<String>,
        tool: impl Into<String>,
        success: bool,
    ) -> Self {
        Self::ToolCompleted {
            branch: branch.into(),
            tool: tool.into(),
            success,
        }
    }

    /// Creates a child-start event.
    ///
    /// - `branch`: Stable parent branch correlation identity.
    ///
    /// A bounded runtime event.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn child_branch_started(branch: impl Into<String>) -> Self {
        Self::ChildBranchStarted {
            branch: branch.into(),
        }
    }

    /// Creates a child-completion event.
    ///
    /// - `branch`: Stable parent branch correlation identity.
    /// - `success`: Whether the child completed successfully.
    ///
    /// A bounded runtime event.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn child_branch_completed(branch: impl Into<String>, success: bool) -> Self {
        Self::ChildBranchCompleted {
            branch: branch.into(),
            success,
        }
    }

    /// Creates a bounded result-produced event.
    ///
    /// - `branch`: Stable branch correlation identity.
    /// - `summary`: Externally meaningful result summary.
    ///
    /// A bounded runtime event.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn branch_result_produced(branch: impl Into<String>, summary: impl Into<String>) -> Self {
        let mut summary = summary.into();
        summary.truncate(256);
        Self::BranchResultProduced {
            branch: branch.into(),
            summary,
        }
    }

    /// Creates a successful branch-completion event.
    ///
    /// - `branch`: Stable branch correlation identity.
    ///
    /// A bounded runtime event.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn branch_execution_completed(branch: impl Into<String>) -> Self {
        Self::BranchExecutionCompleted {
            branch: branch.into(),
        }
    }

    /// Creates a bounded branch-failure event.
    ///
    /// - `branch`: Stable branch correlation identity.
    /// - `category`: Safe failure category.
    ///
    /// A bounded runtime event.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn branch_execution_failed(branch: impl Into<String>, category: impl Into<String>) -> Self {
        Self::BranchExecutionFailed {
            branch: branch.into(),
            category: category.into(),
        }
    }

    /// Creates a branch-cancellation event.
    ///
    /// - `branch`: Stable branch correlation identity.
    ///
    /// A bounded runtime event.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn branch_execution_cancelled(branch: impl Into<String>) -> Self {
        Self::BranchExecutionCancelled {
            branch: branch.into(),
        }
    }

    /// Returns the stable branch correlation identity.
    ///
    /// The branch identity carried by this event.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn branch(&self) -> &str {
        match self {
            Self::BranchExecutionStarted { branch }
            | Self::PhaseChanged { branch, .. }
            | Self::ModelInvocationStarted { branch, .. }
            | Self::ModelTextDelta { branch, .. }
            | Self::ReasoningSummary { branch, .. }
            | Self::ToolStarted { branch, .. }
            | Self::ToolCompleted { branch, .. }
            | Self::ChildBranchStarted { branch }
            | Self::ChildBranchCompleted { branch, .. }
            | Self::BranchResultProduced { branch, .. }
            | Self::BranchExecutionCompleted { branch }
            | Self::BranchExecutionFailed { branch, .. }
            | Self::BranchExecutionCancelled { branch } => branch,
        }
    }
}
