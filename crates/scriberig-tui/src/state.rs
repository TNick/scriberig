//! Stores presentation-only state for the initial terminal shell.

use crate::{
    action::Action,
    effect::Effect,
    event::TuiEvent,
    explorer::ExplorerModel,
    inspector::{InspectorModel, LoadState},
    navigation::NavigationState,
    overlay::{Overlay, input::InputState},
    selection::EntitySelection,
};
use scriberig_app::RuntimeCapability;

/// The visible shell pane receiving keyboard focus.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Focus {
    /// The semantic explorer pane.
    Explorer,
    /// The primary content pane.
    Primary,
    /// The inspector pane.
    Inspector,
}

/// Describes the presentation connection to the semantic service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionState {
    /// The service has not completed its first request.
    Connecting,
    /// Requests are completing normally.
    Connected,
    /// A previous connection was lost and cached data may be stale.
    Disconnected,
    /// A connection request failed with a visible error.
    Failed,
}

impl Focus {
    /// Moves focus through the shell's stable pane order.
    /// - `reverse`: Argument named `reverse`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    fn next(self, reverse: bool) -> Self {
        let panes = [Self::Explorer, Self::Primary, Self::Inspector];
        let index = panes.iter().position(|pane| *pane == self).unwrap_or(1);
        panes[(index + if reverse { 2 } else { 1 }) % panes.len()]
    }
}

/// Runtime and presentation state for the initial TUI shell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiState {
    /// Whether the event loop should terminate.
    pub should_quit: bool,
    /// The most recently observed terminal dimensions.
    pub terminal_size: (u16, u16),
    /// The pane that currently owns local keyboard focus.
    pub focus: Focus,
    /// A short status message rendered in the shell.
    pub shell_message: String,
    /// Current semantic-service connection state.
    pub connection: ConnectionState,
    /// Whether cached models require authoritative refresh after reconnect.
    pub stale: bool,
    /// Semantic navigation history, independent from pane focus.
    pub navigation: NavigationState,
    /// The active modal overlay, if one is open.
    pub overlay: Option<Overlay>,
    /// Presentation cursor index.
    pub cursor: usize,
    /// Stable semantic selection independent of the visible cursor.
    pub selection: Option<EntitySelection>,
    /// Semantic explorer model.
    pub explorer: ExplorerModel,
    /// Current inspector request state.
    pub inspector: LoadState<InspectorModel>,
    /// Runtime capabilities supplied by the application adapter.
    pub capabilities: Vec<RuntimeCapability>,
    /// Next request identifier.
    pub next_request_id: u64,
    /// Selected primary view.
    pub primary_view: crate::selection::PrimaryView,
    /// History request/result state.
    pub history: LoadState<crate::view::history::HistoryModel>,
    /// Diff request/result state.
    pub diff: LoadState<crate::view::diff::DiffModel>,
    /// Projection request/result state.
    pub projection: LoadState<crate::view::projection::ProjectionModel>,
    /// Change review request/result state.
    pub review: LoadState<crate::view::changes::ChangeReviewModel>,
    /// Last operation failure retained for review and refresh feedback.
    pub operation_error: Option<(String, String)>,
    /// Last reviewed proposal retained after a failed confirmation.
    pub review_proposal: Option<crate::view::changes::ChangeReviewModel>,
    /// Operation observations retained for the operations view.
    pub operations: Vec<crate::view::operations::OperationModel>,
    /// Session tree request/result state.
    pub session: LoadState<crate::view::session::SessionModel>,
    /// Operation identities with an authoritative refresh already requested.
    pub pending_operation_refresh: Vec<scriberig_core::OperationId>,
    /// Bounded stream/output handles shown as in-progress observations.
    pub agent_output_observations: Vec<String>,
}

impl Default for TuiState {
    /// Creates an empty, deterministic shell state.
    ///
    /// # Returns
    /// The value produced by the declaration.
    fn default() -> Self {
        Self {
            should_quit: false,
            terminal_size: (80, 24),
            focus: Focus::Primary,
            shell_message: "No semantic service is attached in this runtime.".to_owned(),
            connection: ConnectionState::Connecting,
            stale: false,
            navigation: NavigationState::default(),
            overlay: None,
            cursor: 0,
            selection: None,
            explorer: ExplorerModel::default(),
            inspector: LoadState::NotRequested,
            capabilities: Vec::new(),
            next_request_id: 1,
            primary_view: crate::selection::PrimaryView::Overview,
            history: LoadState::NotRequested,
            diff: LoadState::NotRequested,
            projection: LoadState::NotRequested,
            review: LoadState::NotRequested,
            operation_error: None,
            review_proposal: None,
            operations: Vec::new(),
            session: LoadState::NotRequested,
            pending_operation_refresh: Vec::new(),
            agent_output_observations: Vec::new(),
        }
    }
}

impl TuiState {
    /// Applies one local terminal event to presentation state.
    /// - `event`: Argument named `event`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn apply_event(&mut self, event: &TuiEvent) -> Vec<Effect> {
        match event {
            TuiEvent::Key(key) => {
                if let Some(action) = crate::keymap::action_for(self, key) {
                    return reduce(self, action);
                }
            }
            TuiEvent::Resize { width, height } => {
                return reduce(
                    self,
                    Action::TerminalResized {
                        width: *width,
                        height: *height,
                    },
                );
            }
            TuiEvent::Shutdown => return reduce(self, Action::QuitRequested),
            TuiEvent::Paste(text) => {
                return reduce(self, Action::Paste(text.clone()));
            }
            TuiEvent::Application(event) => return reduce_application_event(self, event),
            TuiEvent::Tick => {}
        }
        Vec::new()
    }

    /// Reports whether a text input currently owns keyboard focus.
    #[must_use]
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn is_input_active(&self) -> bool {
        matches!(self.overlay, Some(Overlay::Input(_) | Overlay::Composer(_)))
    }

    /// Returns the current input text, when an input overlay is active.
    #[must_use]
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn input_text(&self) -> Option<&str> {
        match self.overlay.as_ref() {
            Some(Overlay::Input(input)) => Some(&input.text),
            Some(Overlay::Composer(composer)) => Some(&composer.text),
            _ => None,
        }
    }
}

#[path = "state_actions.rs"]
mod state_actions;
#[path = "state_cursor_reducer.rs"]
mod state_cursor_reducer;
#[path = "state_reducer.rs"]
mod state_reducer;
#[path = "state_reducer_tail.rs"]
mod state_reducer_tail;
use state_actions::reduce_application_event;
pub use state_reducer::reduce;

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
