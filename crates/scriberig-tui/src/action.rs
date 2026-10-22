//! Defines typed local interaction intents for the TUI reducer.

/// Direction for cursor movement in a list-like presentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    /// Move toward the previous item.
    Previous,
    /// Move toward the next item.
    Next,
}

/// A typed presentation action independent of terminal or service handles.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Action {
    /// Records that an effect had no stateful result.
    Noop,
    /// Request normal runtime termination.
    QuitRequested,
    /// Close the active overlay, if any.
    OverlayClosed,
    /// Move a local cursor.
    CursorMoved(Direction),
    /// Activate the focused row or overlay item.
    Activated,
    /// Navigate to the previous semantic presentation location.
    Back,
    /// Navigate to the next semantic presentation location.
    Forward,
    /// Request a presentation refresh.
    RefreshRequested,
    /// Open the help overlay.
    HelpOpened,
    /// Open the local command palette.
    PaletteOpened,
    /// Open the find input overlay.
    FindOpened,
    /// Open the exact snapshot chooser for semantic comparison.
    DiffChooserOpened,
    /// Open a source projection for the exact selected snapshot.
    ProjectionOpened,
    /// Update the active text input.
    InputChanged(String),
    /// Submit the active text input.
    InputSubmitted,
    /// Applies an accepted conversation submission with authoritative handles.
    ConversationAgentAccepted(crate::service::AgentHandle),
    /// Enters or leaves conversation-tree browse mode.
    ConversationBrowsing(bool),
    /// Pins the current navigable conversation source as the reply target.
    ConversationPinCursor,
    /// Resumes local follow of the shared plan coordinator cursor.
    ConversationFollowCoordinator,
    /// Expands or collapses the currently selected branch in the local tree.
    ConversationToggleExpansion,
    /// Insert one paste payload into the active input.
    Paste(String),
    /// Activate a concrete semantic location.
    ActivateLocation(crate::selection::NavigationLocation),
    /// Mark an inspect request as outstanding.
    InspectRequested {
        /// Monotonic request identifier.
        request_id: u64,
        /// Entity being inspected.
        entity: scriberig_core::EntityId,
        /// Optional exact snapshot.
        snapshot: Option<scriberig_core::SnapshotId>,
        /// Whether this request should switch the primary view to Overview.
        switch_view: bool,
    },
    /// Apply a successful inspect result.
    InspectSucceeded {
        /// Request that produced the result.
        request_id: u64,
        /// Presentation projection of the result.
        model: crate::inspector::InspectorModel,
    },
    /// Apply a failed inspect result.
    InspectFailed {
        /// Request that failed.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Applies a successful bounded Explorer listing.
    ExplorerSucceeded {
        /// Request that produced the rows.
        request_id: u64,
        /// Snapshot-bound semantic rows.
        rows: Vec<crate::explorer::ExplorerRow>,
        /// Opaque continuation for another bounded page.
        next: Option<scriberig_query::ContinuationToken>,
    },
    /// Apply runtime capability metadata returned by the application.
    CapabilitiesLoaded(Vec<scriberig_app::RuntimeCapability>),
    /// Replace observed lifecycle state with a current application snapshot.
    OperationsLoaded(Vec<scriberig_app::OperationRecord>),
    /// Switches the primary view without changing semantic selection.
    ViewSelected(crate::selection::PrimaryView),
    /// Applies a history operation result.
    HistorySucceeded {
        /// Request generation.
        request_id: u64,
        /// Projected history.
        model: crate::view::history::HistoryModel,
    },
    /// Applies a diff operation result.
    DiffSucceeded {
        /// Request generation.
        request_id: u64,
        /// Projected diff.
        model: crate::view::diff::DiffModel,
    },
    /// Applies a projection operation result.
    ProjectionSucceeded {
        /// Request generation.
        request_id: u64,
        /// Projected rendered content.
        model: crate::view::projection::ProjectionModel,
    },
    /// Applies a candidate review result.
    ReviewSucceeded {
        /// Request generation.
        request_id: u64,
        /// Projected review.
        model: crate::view::changes::ChangeReviewModel,
    },
    /// Records a committed candidate and refreshes the active semantic selection.
    CommitSucceeded {
        /// Request generation.
        request_id: u64,
        /// Reviewed candidate result.
        model: crate::view::changes::ChangeReviewModel,
        /// Authoritative successor snapshot returned by the application.
        snapshot: scriberig_core::SnapshotId,
    },
    /// Applies a failed operation result without retrying it.
    OperationFailed {
        /// Request generation.
        request_id: u64,
        /// Stable application error category.
        category: String,
        /// Safe error message.
        message: String,
    },
    /// Opens the explicit review confirmation overlay.
    ConfirmReview,
    /// Cancels review confirmation without a service call.
    CancelReview,
    /// Confirms the reviewed canonical commit.
    CommitConfirmed(crate::view::changes::ConfirmedCommit),
    /// Requests history for an exact snapshot.
    HistoryRequested {
        /// Exact snapshot whose history is requested.
        snapshot: scriberig_core::SnapshotId,
    },
    /// Requests snapshot-scoped semantic discovery.
    FindRequested(String),
    /// Applies a successful semantic discovery result.
    FindSucceeded {
        /// Request generation.
        request_id: u64,
        /// Projected discovery rows.
        model: crate::view::find::FindModel,
    },
    /// Requests a two-sided semantic diff.
    DiffRequested {
        /// Public exact diff request.
        request: scriberig_app::DiffRequest,
    },
    /// Requests a shared projection render.
    ProjectionRequested {
        /// Public render request.
        request: scriberig_app::RenderRequest,
    },
    /// Requests candidate construction for review.
    ReviewRequested {
        /// Public apply request.
        request: scriberig_app::ApplyRequest,
    },
    /// Applies an authoritative operation status observation.
    OperationObserved(crate::view::operations::OperationModel),
    /// Requests an authoritative operation status refresh.
    OperationStatusRequested(scriberig_core::OperationId),
    /// Requests application-owned cancellation of an operation.
    OperationCancelRequested(scriberig_core::OperationId),
    /// Applies a public session tree result.
    SessionSucceeded {
        /// Request generation.
        request_id: u64,
        /// Projected session tree.
        model: crate::view::session::SessionModel,
        /// Tree-aware conversation projection over the same durable snapshot.
        conversation: crate::view::conversation::ConversationModel,
        /// Shared session snapshot retained for presentation-only rebuilding.
        session_snapshot: Option<scriberig_session::Session>,
    },
    /// Applies a durable list of sessions to the session-list overlay.
    SessionsListed(Vec<scriberig_session::Session>),
    /// Applies an exact session switch to the conversation projection.
    SessionSwitched(scriberig_session::Session),
    /// Applies removal of one exact session.
    SessionRemoved(scriberig_core::EntityId),
    /// Applies an authoritative plan result and rebuilds the conversation tree.
    PlanUpdated(scriberig_session::Plan),
    /// Applies a successful model-library read.
    ModelLibraryLoaded {
        /// Request generation.
        request_id: u64,
        /// Credential-safe model and profile descriptors.
        library: scriberig_protocol::ModelLibraryResponse,
    },
    /// Applies shared presentation defaults without replacing local overrides.
    PresentationPreferencesLoaded(scriberig_protocol::AgentPresentationPreferencesResponse),
    /// Applies expanded safe detail for one durable tool result.
    ToolResultDetailLoaded(scriberig_protocol::ToolResultDetailResponse),
    /// Store one captured provider-neutral model request detail response.
    ModelRequestDetailLoaded(scriberig_protocol::ModelRequestDetailResponse),
    /// Applies an authoritative future-invocation execution selection.
    SessionExecutionSelectionUpdated(scriberig_session::SessionExecutionSelection),
    /// Tracks the operation returned by an agent command submission.
    AgentSubmitted(crate::service::AgentHandle),
    /// Requests a public session tree.
    SessionRequested {
        /// Stable session identity.
        session: scriberig_core::EntityId,
        /// Optional branch whose activity is loaded on demand.
        branch: Option<scriberig_core::EntityId>,
    },
    /// Opens the agent composer for an explicit session branch.
    ComposerOpened {
        /// Session receiving the command.
        session: scriberig_core::EntityId,
        /// Optional branch receiving the new turn.
        branch: Option<scriberig_core::EntityId>,
    },
    /// Opens the persistent composer on the session surface.
    SessionComposerOpened {
        /// Session receiving the command.
        session: scriberig_core::EntityId,
        /// Optional branch receiving the new turn.
        branch: Option<scriberig_core::EntityId>,
    },
    /// Changes keyboard focus for the persistent session composer.
    SessionComposerFocused(bool),
    /// Records successful contact after a remote disconnect.
    ConnectionRestored,
    /// Records bounded agent output under its shared operation identity.
    AgentOutputObserved {
        /// Operation producing this output.
        operation: scriberig_core::OperationId,
        /// Output fragment or complete output value.
        text: String,
        /// Whether the shared event contains the complete output value.
        complete: bool,
    },
}
