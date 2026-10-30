//! Defines work requested by the reducer but executed by the runtime.

/// A presentation or future semantic effect emitted by state reduction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Effect {
    /// No work is required outside the reducer.
    None,
    /// Ask the owning service adapter to refresh presentation data.
    RequestRefresh,
    /// Ask the owning service adapter to execute a find request.
    RequestFind {
        /// Monotonic request identifier.
        request_id: u64,
        /// Query text supplied by the user.
        text: String,
        /// Exact snapshot scope for discovery.
        snapshot: scriberig_core::SnapshotId,
    },
    /// Ask the owning service adapter to execute a contextual action later.
    RequestSemanticAction {
        /// Stable semantic action identifier.
        id: String,
        /// Explicit action inputs.
        inputs: Vec<(String, String)>,
    },
    /// Request runtime termination.
    RequestQuit,
    /// Request semantic inspection through the TUI service port.
    InspectEntity {
        /// Monotonic request identifier.
        request_id: u64,
        /// Entity to inspect.
        entity: scriberig_core::EntityId,
        /// Optional exact snapshot.
        snapshot: Option<scriberig_core::SnapshotId>,
    },
    /// Request the bounded semantic children of one snapshot entity.
    ListExplorer {
        /// Monotonic request identifier.
        request_id: u64,
        /// Entity whose snapshot collection is listed.
        entity: scriberig_core::EntityId,
        /// Exact snapshot containing the collection.
        snapshot: scriberig_core::SnapshotId,
        /// Opaque continuation for the next page.
        continuation: Option<scriberig_query::ContinuationToken>,
    },
    /// Request the current runtime capability descriptors.
    RequestCapabilities,
    /// Read exact semantic history.
    History {
        /// Request generation.
        request_id: u64,
        /// Exact snapshot context.
        snapshot: scriberig_core::SnapshotId,
    },
    /// Compute a structured two-sided semantic diff.
    Diff {
        /// Request generation.
        request_id: u64,
        /// Public exact diff request.
        request: scriberig_app::DiffRequest,
    },
    /// Render through a shared projection operation.
    Render {
        /// Request generation.
        request_id: u64,
        /// Public render request.
        request: scriberig_app::RenderRequest,
    },
    /// Apply a candidate change without promotion.
    Apply {
        /// Request generation.
        request_id: u64,
        /// Public apply request.
        request: scriberig_app::ApplyRequest,
    },
    /// Commit a reviewed change with its exact base context.
    Commit {
        /// Request generation.
        request_id: u64,
        /// Public commit request.
        request: scriberig_app::CommitRequest,
    },
    /// Read the authoritative lifecycle record for one operation.
    OperationStatus {
        /// Operation identity to inspect.
        operation: scriberig_core::OperationId,
    },
    /// Ask the application to cancel one operation.
    CancelOperation {
        /// Operation identity to cancel.
        operation: scriberig_core::OperationId,
    },
    /// Refresh the tracked application operation records.
    ListOperations,
    /// Read a public session tree.
    ReadSession {
        /// Request generation.
        request_id: u64,
        /// Stable session identity.
        session: scriberig_core::EntityId,
        /// Optional branch whose activity is loaded on demand.
        branch: Option<scriberig_core::EntityId>,
    },
    /// List the caller's durable conversation sessions.
    ListSessions,
    /// Read and select one exact durable session.
    SwitchSession {
        /// Stable session identity to load.
        session: scriberig_core::EntityId,
    },
    /// Remove one exact idle durable session.
    RemoveSession {
        /// Stable session identity to remove.
        session: scriberig_core::EntityId,
    },
    /// Read safe upstream-derived model display rows.
    ReadModelLibrary {
        /// Request generation.
        request_id: u64,
    },
    /// Read shared presentation defaults for this trusted application user.
    ReadPresentationPreferences,
    /// Read expanded safe detail for one durable tool result.
    ReadToolResultDetail {
        /// Session containing the result.
        session: scriberig_core::EntityId,
        /// Branch containing the result.
        branch: scriberig_core::EntityId,
        /// Stable tool-result identity.
        result: scriberig_core::EntityId,
    },
    /// Read captured safe request detail for one model invocation.
    ReadModelRequestDetail {
        /// Session containing the invocation.
        session: scriberig_core::EntityId,
        /// Branch containing the invocation.
        branch: scriberig_core::EntityId,
        /// Stable model-invocation identity.
        invocation: scriberig_core::EntityId,
    },
    /// Select a model profile for future invocations in one session.
    SetSessionModel(scriberig_app::SessionModelSelectionRequest),
    /// Select a thinking level for future invocations in one session.
    SetSessionThinkingLevel(scriberig_app::SessionThinkingSelectionRequest),
    /// Submit a command to a session branch through the application boundary.
    SubmitAgent {
        /// Stable session identity.
        session: scriberig_core::EntityId,
        /// Optional target branch.
        branch: Option<scriberig_core::EntityId>,
        /// User-authored command text.
        prompt: String,
    },
    /// Starts a new session through shared application semantics.
    StartNewConversation {
        /// First user-authored instruction.
        prompt: String,
        /// Exact semantic project selection for the initial model context.
        selection: Option<scriberig_app::AgentSelection>,
    },
    /// Continues at one exact durable conversation anchor.
    ContinueConversation {
        /// Owning session identity.
        session: scriberig_core::EntityId,
        /// Stable semantic continuation target.
        anchor: scriberig_session::ContinuationAnchor,
        /// User-authored follow-up instruction.
        prompt: String,
    },
    /// Creates a durable plan through the shared application plan operation.
    CreatePlan(scriberig_app::PlanCreateRequest),
    /// Applies a complete structured revision to a durable plan.
    Replan(scriberig_app::PlanReplanRequest),
    /// Changes a durable plan's lifecycle through shared application semantics.
    PlanLifecycle {
        /// Plan identity and expected durable revision.
        request: scriberig_app::PlanLifecycleRequest,
        /// Requested lifecycle transition.
        action: scriberig_app::PlanLifecycleAction,
    },
    /// Changes one plan node's credential-safe model-library assignment.
    SetPlanNodeAssignment(scriberig_app::PlanAssignmentRequest),
    /// Retries one failed plan node at its observed plan revision.
    RetryPlanNode(scriberig_app::PlanRetryRequest),
    /// Adds ordinary work to a running plan at its observed revision.
    AddPlanNextStep(scriberig_app::PlanNextStepRequest),
    /// Adds required structural children to a selected plan node.
    DecomposePlanNode(scriberig_app::PlanDecomposeRequest),
    /// Applies one typed summary, evidence, gate, or waiver mutation.
    MutatePlanSemantics(scriberig_app::PlanSemanticMutationRequest),
    /// Run the deterministic semantic project agent for the selected entity.
    ProjectAgent {
        /// Monotonic request identifier.
        request_id: u64,
        /// Selected semantic entity.
        entity: scriberig_core::EntityId,
        /// Exact base snapshot.
        snapshot: scriberig_core::SnapshotId,
        /// User instruction sent to the provider.
        prompt: String,
    },
}
