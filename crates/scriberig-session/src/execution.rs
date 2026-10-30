//! Defines provider-neutral model execution records and deterministic activity order.

#[path = "execution/request_detail.rs"]
mod request_detail;
pub use request_detail::*;

#[path = "execution/provenance.rs"]
mod provenance;
pub use provenance::*;

/// Session-scoped model configuration for future invocations.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SessionExecutionSelection {
    /// Monotonic mutation revision, independent from event cursors.
    pub revision: u64,
    /// Selected upstream provider identity.
    pub provider_id: Option<String>,
    /// Selected upstream provider-scoped model identity.
    pub model_id: Option<String>,
    /// Selected model-advertised thinking-level identity.
    pub thinking_level: Option<String>,
    /// Bounded replayable model and thinking selection provenance.
    #[serde(default)]
    pub history: Vec<SessionExecutionChange>,
}

/// Records one durable future-invocation selection change.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SessionExecutionChange {
    /// Selection mutation revision that produced this record.
    pub revision: u64,
    /// Semantic category of the changed selection.
    pub kind: AgentActivityKind,
    /// Safe concise description of the resulting selection.
    pub summary: String,
    /// Resolved upstream provider identity after the change, when selected.
    pub provider_id: Option<String>,
    /// Resolved upstream model identity after the change, when selected.
    pub model_id: Option<String>,
    /// Resolved thinking level after the change, when selected.
    pub thinking_level: Option<String>,
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, schemars::JsonSchema, serde::Deserialize, serde::Serialize,
)]
/// Provider-neutral execution activity kinds shared by persistence and events.
///
/// # Public members
/// Each variant identifies one shared semantic activity kind.
///
/// # Private members
/// None.
pub enum SessionActivityKind {
    /// Human supplied a prompt or instruction.
    HumanInput,
    /// Externally meaningful model output was committed.
    AssistantOutput,
    /// Safe user-facing reasoning summary.
    ReasoningSummary,
    /// Scriberig runtime phase or status.
    RuntimeStatus,
    /// Model request was issued.
    ModelInvocation,
    /// Safe captured model request detail.
    ModelRequest,
    /// A trusted tool was invoked.
    ToolInvocation,
    /// A tool returned or failed.
    ToolResult,
    /// A child branch was created.
    BranchStarted,
    /// A child branch completed.
    BranchCompleted,
    /// Operation progress changed.
    OperationProgress,
    /// Validation completed.
    Validation,
    /// Future model selection changed.
    ModelSelectionChanged,
    /// Future thinking-level selection changed.
    ThinkingLevelChanged,
    /// A durable plan was created.
    PlanCreated,
    /// A new durable plan revision was accepted.
    PlanRevised,
    /// A plan node became ready for worker execution.
    PlanNodeReady,
    /// A plan node acquired an isolated worker branch.
    PlanNodeStarted,
    /// A plan node completed with an explicit result.
    PlanNodeCompleted,
    /// A plan node failed and continues to block dependents.
    PlanNodeFailed,
    /// A plan assignment changed for a future invocation.
    AssignmentChanged,
    /// Coordinator ownership or lifecycle changed.
    CoordinatorChanged,
    /// A dependency edge changed fulfillment state.
    DependencyStateChanged,
    /// A typed semantic domain operation was recorded.
    SemanticOperation,
    /// An explicitly authorized external process ran.
    ExternalProcess,
    /// An operation or activity failed.
    Error,
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, schemars::JsonSchema, serde::Deserialize, serde::Serialize,
)]
/// Lifecycle state of shared execution activity.
///
/// # Public members
/// Each variant identifies one activity lifecycle state.
///
/// # Private members
/// None.
pub enum SessionActivityState {
    /// Activity remains in progress.
    Running,
    /// Activity completed successfully.
    Completed,
    /// Activity completed with a failure.
    Failed,
    /// Activity was cancelled.
    Cancelled,
}

/// Shared application-facing name for session activity categories.
pub type AgentActivityKind = SessionActivityKind;

/// Shared application-facing name for session activity lifecycle states.
pub type AgentActivityState = SessionActivityState;

/// Trusted semantic category of a model-visible application tool.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    schemars::JsonSchema,
    serde::Deserialize,
    serde::Serialize,
)]
/// Identifies a semantic or generic tool activity in durable execution history.
pub enum ToolActivityKind {
    /// Reads entities, relations, or semantic projections.
    SemanticRead,
    /// Applies a typed domain mutation through shared change operations.
    SemanticMutation,
    /// Changes a derived projection without changing canonical meaning.
    ProjectionOperation,
    /// Runs validation or another explicitly authorized external process.
    ValidationProcess,
    /// Uses generic interoperability or bootstrap access to opaque content.
    OpaqueFallback,
    /// Creates an isolated child branch for separate work.
    BranchProducing,
}

impl SessionExecutionSelection {
    /// Replaces the future-invocation selection using optimistic concurrency.
    ///
    /// - `self`: Current session selection.
    /// - `expected_revision`: Revision observed by the mutating client.
    /// - `provider_id`: Upstream provider identity, when a model is selected.
    /// - `model_id`: Upstream provider-scoped model identity, when selected.
    /// - `thinking_level`: Supported thinking level, when selected.
    ///
    /// The updated selection with its next mutation revision.
    ///
    /// Returns a conflict for stale revisions or inconsistent absent values.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn replace(
        &mut self,
        expected_revision: u64,
        provider_id: Option<String>,
        model_id: Option<String>,
        thinking_level: Option<String>,
    ) -> Result<(), SelectionError> {
        if expected_revision != self.revision {
            return Err(SelectionError::RevisionConflict {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        if provider_id.is_some() != model_id.is_some()
            || (model_id.is_none() && thinking_level.is_some())
        {
            return Err(SelectionError::InconsistentSelection);
        }
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(SelectionError::RevisionExhausted)?;
        self.provider_id = provider_id;
        self.model_id = model_id;
        self.thinking_level = thinking_level;
        Ok(())
    }
}

/// Describes optimistic-concurrency failures for execution selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionError {
    /// The caller's observed revision is stale.
    RevisionConflict { expected: u64, actual: u64 },
    /// Provider and model must be set or cleared together.
    InconsistentSelection,
    /// The selection revision has no representable successor.
    RevisionExhausted,
}
