//! Stores presentation-only state for the initial terminal shell.

use std::collections::HashSet;

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

/// Controls client-local tool-result presentation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ToolOutputVisibility {
    /// Replace summaries and detail with a hidden marker.
    Hidden,
    /// Show only the safe result summary.
    #[default]
    Summary,
    /// Fetch and show bounded allowlisted detail.
    Full,
}

/// Controls whether tool calls use compact or structured presentation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ToolInvocationVisibility {
    /// Show the compact command-like tool label.
    #[default]
    Simple,
    /// Show bounded schema-redacted arguments below the tool label.
    Detailed,
}

/// Controls client-local visibility of captured model requests.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PromptVisibility {
    /// Omit captured request sections from the activity view.
    #[default]
    Hidden,
    /// Show bounded captured request sections for model invocations.
    Shown,
}

/// Identifies a presentation setting overridden in the current TUI session.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum PresentationControl {
    /// Tool-result visibility override.
    ToolOutput,
    /// Tool-invocation visibility override.
    ToolInvocation,
    /// Captured model-request visibility override.
    Prompt,
    /// Reasoning-summary visibility override.
    Reasoning,
}

/// Controls client-local visibility of safe reasoning summaries and status.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReasoningVisibility {
    /// Omit safe reasoning summaries and runtime status rows.
    #[default]
    Hidden,
    /// Show safe user-facing summaries and runtime status rows.
    Shown,
}

/// One bounded live activity retained for presentation-only rerendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransientAgentActivity {
    /// Runtime branch associated with the activity.
    pub branch: String,
    /// Provider-neutral activity category.
    pub activity_kind: String,
    /// Lifecycle state such as running or completed.
    pub state: String,
    /// Safe bounded user-facing summary.
    pub summary: String,
}

/// Persistent draft and target displayed on the session surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionComposerState {
    /// Session receiving the next submitted command.
    pub session: scriberig_core::EntityId,
    /// Optional branch receiving the next submitted command.
    pub branch: Option<scriberig_core::EntityId>,
    /// Multiline user-authored draft.
    pub text: String,
    /// Prompt awaiting acknowledgement by the application adapter.
    pub pending_prompt: Option<String>,
    /// Safe submission failure displayed with the draft.
    pub error: Option<String>,
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
    /// Stable active project identity, when selected.
    pub project: Option<String>,
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
    /// Find request/result state.
    pub find: LoadState<crate::view::find::FindModel>,
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
    /// Bounded runtime activity retained independently of local visibility.
    pub transient_agent_activities: Vec<TransientAgentActivity>,
    /// Operation observations retained for the operations view.
    pub operations: Vec<crate::view::operations::OperationModel>,
    /// Session tree request/result state.
    pub session: LoadState<crate::view::session::SessionModel>,
    /// Credential-safe model library available to the interactive picker.
    pub model_library: LoadState<scriberig_protocol::ModelLibraryResponse>,
    /// Whether a pending model-library read was opened from thinking controls.
    pub thinking_picker_pending: Option<()>,
    /// Branch to reveal after the matching agent submission finishes.
    pub session_branch_focus: Option<scriberig_core::EntityId>,
    /// Operation identities with an authoritative refresh already requested.
    pub pending_operation_refresh: Vec<scriberig_core::OperationId>,
    /// Bounded stream/output handles shown as in-progress observations.
    pub agent_output_observations: Vec<String>,
    /// Stable session, branch, and operation handles for active agent work.
    pub agent_handles: Vec<crate::service::AgentHandle>,
    /// Persistent session composer draft, when an agent session is active.
    pub session_composer: Option<SessionComposerState>,
    /// Whether the persistent composer owns keyboard input.
    pub session_composer_focused: bool,
    /// Client-local tool-result verbosity.
    pub tool_output_visibility: ToolOutputVisibility,
    /// Client-local tool-call presentation mode.
    pub tool_invocation_visibility: ToolInvocationVisibility,
    /// Client-local prompt/model-request visibility.
    pub prompt_visibility: PromptVisibility,
    /// Client-local reasoning-summary visibility.
    pub reasoning_visibility: ReasoningVisibility,
    /// Shared preference revision applied to the TUI presentation layer.
    pub presentation_preferences_revision: Option<u64>,
    /// Presentation controls whose shared defaults are locally overridden.
    pub(crate) presentation_overrides: HashSet<PresentationControl>,
    /// Captured safe request detail indexed by model invocation identity.
    pub model_request_details: std::collections::BTreeMap<
        scriberig_core::EntityId,
        scriberig_protocol::ModelRequestDetailResponse,
    >,
    /// Expanded safe detail cached by durable tool-result identity.
    pub tool_result_details: std::collections::BTreeMap<
        scriberig_core::EntityId,
        scriberig_protocol::ToolResultDetailResponse,
    >,
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
            project: None,
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
            find: LoadState::NotRequested,
            diff: LoadState::NotRequested,
            projection: LoadState::NotRequested,
            review: LoadState::NotRequested,
            operation_error: None,
            review_proposal: None,
            transient_agent_activities: Vec::new(),
            operations: Vec::new(),
            session: LoadState::NotRequested,
            model_library: LoadState::NotRequested,
            thinking_picker_pending: None,
            session_branch_focus: None,
            pending_operation_refresh: Vec::new(),
            agent_output_observations: Vec::new(),
            agent_handles: Vec::new(),
            session_composer: None,
            session_composer_focused: false,
            tool_output_visibility: ToolOutputVisibility::default(),
            tool_invocation_visibility: ToolInvocationVisibility::default(),
            prompt_visibility: PromptVisibility::default(),
            reasoning_visibility: ReasoningVisibility::default(),
            presentation_preferences_revision: None,
            presentation_overrides: HashSet::new(),
            model_request_details: std::collections::BTreeMap::new(),
            tool_result_details: std::collections::BTreeMap::new(),
        }
    }
}

impl TuiState {
    /// Reports whether this TUI session overrides one shared preference.
    ///
    /// # Parameters
    /// - `control`: Presentation control to inspect.
    ///
    /// # Returns
    /// Whether the control has a local override.
    pub(crate) fn has_presentation_override(&self, control: PresentationControl) -> bool {
        self.presentation_overrides.contains(&control)
    }

    /// Records a local override for one presentation control.
    ///
    /// # Parameters
    /// - `control`: Presentation control that was changed locally.
    pub(crate) fn set_presentation_override(&mut self, control: PresentationControl) {
        self.presentation_overrides.insert(control);
    }

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
            || (self.session_composer.is_some() && self.session_composer_focused)
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
            None => self
                .session_composer
                .as_ref()
                .map(|composer| composer.text.as_str()),
            _ => None,
        }
    }
}

#[path = "state_actions.rs"]
mod state_actions;
#[path = "state_cursor_reducer.rs"]
mod state_cursor_reducer;
#[path = "state_model_picker.rs"]
mod state_model_picker;
#[path = "state_presentation.rs"]
mod state_presentation;
#[path = "state_reducer.rs"]
mod state_reducer;
#[path = "state_reducer_tail.rs"]
mod state_reducer_tail;
use state_actions::reduce_application_event;
pub(super) use state_model_picker::{show_model_picker, show_thinking_picker};
pub(super) use state_presentation::activate_palette_control;
pub use state_reducer::reduce;

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
