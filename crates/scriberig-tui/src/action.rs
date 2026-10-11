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
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Action {
    /// Records that an effect had no stateful result.
    Noop,
    /// Request normal runtime termination.
    QuitRequested,
    /// Close the active overlay, if any.
    OverlayClosed,
    /// Move focus to the next visible pane.
    FocusNext,
    /// Move focus to the previous visible pane.
    FocusPrevious,
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
    /// Update the active text input.
    InputChanged(String),
    /// Submit the active text input.
    InputSubmitted,
    /// Record a terminal resize.
    TerminalResized {
        /// New terminal width in columns.
        width: u16,
        /// New terminal height in rows.
        height: u16,
    },
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
    /// Apply runtime capability metadata returned by the application.
    CapabilitiesLoaded(Vec<scriberig_app::RuntimeCapability>),
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
}
