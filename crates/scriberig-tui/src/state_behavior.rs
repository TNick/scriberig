//! Implements presentation state defaults and local event behavior.

use super::{
    Action, ConnectionState, ConversationComposerState, ConversationTarget, Effect, ExplorerModel,
    Focus, HashSet, LoadState, NavigationState, Overlay, PresentationControl, PromptVisibility,
    ReasoningVisibility, ToolInvocationVisibility, ToolOutputVisibility, TuiEvent, TuiState,
    reduce, reduce_application_event,
};
use std::collections::BTreeSet;

impl Default for TuiState {
    /// Creates an empty, deterministic shell state.
    ///
    /// # Returns
    /// The operation result.
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
            agent_selection: None,
            explorer: ExplorerModel::default(),
            inspector: LoadState::NotRequested,
            capabilities: Vec::new(),
            next_request_id: 1,
            primary_view: crate::selection::PrimaryView::Conversation,
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
            conversation: None,
            session_snapshot: None,
            conversation_composer: ConversationComposerState {
                target: ConversationTarget::NewSession,
                draft: String::new(),
                pending: false,
                error: None,
            },
            conversation_cursor: None,
            conversation_browsing: false,
            coordinator_cursor: None,
            follow_coordinator: true,
            expanded_branches: BTreeSet::new(),
            model_library: LoadState::NotRequested,
            thinking_picker_pending: None,
            plan_assignment_target: None,
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
    /// - `control`: Presentation control to inspect.
    ///
    /// Whether the control has a local override.
    /// # Returns
    /// The operation result.
    pub(crate) fn has_presentation_override(&self, control: PresentationControl) -> bool {
        self.presentation_overrides.contains(&control)
    }

    /// Records a local override for one presentation control.
    ///
    /// - `control`: Presentation control that was changed locally.
    pub(crate) fn set_presentation_override(&mut self, control: PresentationControl) {
        self.presentation_overrides.insert(control);
    }

    /// Applies one local terminal event to presentation state.
    /// - `event`: Argument named `event`.
    ///
    /// # Returns
    /// The operation result.
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
    /// The operation result.
    pub fn is_input_active(&self) -> bool {
        matches!(self.overlay, Some(Overlay::Input(_) | Overlay::Composer(_)))
            || (self.session_composer.is_some() && self.session_composer_focused)
    }

    /// Returns the current input text, when an input overlay is active.
    #[must_use]
    ///
    /// # Returns
    /// The operation result.
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

/// Projects durable conversation state with active observations attached to branches.
///
/// - `state`: TUI state containing presentation-only active activity.
/// - `session`: Exact durable session snapshot.
///
/// A tree projection that places live status at stable branch locations.
/// # Returns
/// The operation result.
pub(crate) fn conversation_projection(
    state: &TuiState,
    session: &scriberig_session::Session,
) -> crate::view::conversation::ConversationModel {
    let mut conversation = crate::view::conversation::ConversationModel::from_session_with_expanded(
        session,
        &state.expanded_branches,
    );
    for activity in &state.transient_agent_activities {
        let Ok(branch) = activity.branch.parse::<scriberig_core::EntityId>() else {
            continue;
        };
        let summary = if activity.activity_kind
            == scriberig_session::AgentActivityKind::ReasoningSummary
            && state.reasoning_visibility == ReasoningVisibility::Hidden
        {
            "summary hidden".to_owned()
        } else {
            crate::inspector::sanitize(&activity.summary)
        };
        conversation.insert_transient_activity(
            branch,
            activity.key,
            format!("Live {:?}: {}", activity.activity_kind, summary),
        );
    }
    conversation
}
