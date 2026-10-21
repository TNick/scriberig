//! Implements presentation state defaults and application-event reduction.

use super::{
    ConnectionState, ConversationComposerState, ConversationTarget, Effect, ExplorerModel, HashSet,
    LoadState, NavigationState, Overlay, PresentationControl, PromptVisibility,
    ReasoningVisibility, ToolInvocationVisibility, ToolOutputVisibility, TuiState,
    reduce_application_event,
};
use std::collections::BTreeSet;

impl ConversationComposerState {
    /// Creates a synchronized composer for one exact conversation target.
    ///
    /// - `target`: Session and continuation anchor receiving the draft.
    /// - `draft`: Initial user-authored composer text.
    ///
    /// A composer whose editor and canonical draft begin synchronized.
    /// # Returns
    /// The initialized conversation composer.
    #[must_use]
    pub fn new(target: ConversationTarget, draft: String) -> Self {
        Self {
            target,
            draft,
            pending_prompt: None,
            pending: false,
            error: None,
        }
    }
}

impl Default for TuiState {
    /// Creates an empty, deterministic shell state.
    ///
    /// # Returns
    /// The operation result.
    fn default() -> Self {
        Self {
            should_quit: false,
            shell_message: "No semantic service is attached in this runtime.".to_owned(),
            project: None,
            connection: ConnectionState::Connecting,
            stale: false,
            navigation: NavigationState::default(),
            overlay: None,
            cursor: 0,
            selection: None,
            semantic_row_cursor: None,
            plan_cursor: None,
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
            session_cursor: None,
            conversation: None,
            session_snapshot: None,
            conversation_composer: ConversationComposerState {
                target: ConversationTarget::NewSession,
                draft: String::new(),
                pending_prompt: None,
                pending: false,
                error: None,
            },
            conversation_cursor: None,
            conversation_browsing: false,
            coordinator_cursor: None,
            follow_coordinator: true,
            expanded_branches: BTreeSet::new(),
            model_library: LoadState::NotRequested,
            plan_assignment_target: None,
            session_branch_focus: None,
            pending_operation_refresh: Vec::new(),
            transient_agent_outputs: Vec::new(),
            next_transient_sequence: 1,
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

    /// Applies one shared application event to presentation state.
    ///
    /// - `event`: Authoritative `ScribeRig` application event.
    ///
    /// Effects produced by reducing the event.
    /// # Returns
    /// Semantic effects requested by the event.
    pub(crate) fn apply_application_event(
        &mut self,
        event: &scriberig_app::ApplicationEvent,
    ) -> Vec<Effect> {
        reduce_application_event(self, event)
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

/// One ordered transient row waiting for authoritative session data.
enum TransientConversationRow {
    /// Activity status row correlated to a branch.
    Activity {
        /// Structural parent branch.
        branch: scriberig_core::EntityId,
        /// Stable activity identity.
        key: u64,
        /// Safe user-facing label.
        label: String,
    },
    /// Partial or complete assistant output correlated to an operation.
    Output {
        /// Structural parent branch.
        branch: scriberig_core::EntityId,
        /// Stable shared operation identity.
        operation: scriberig_core::OperationId,
        /// Safe bounded output text.
        text: String,
    },
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
    attach_transient_rows(state, &mut conversation, Some(session));
    conversation
}

/// Rebuilds the active conversation from durable state and live observations.
///
/// - `state`: Presentation state receiving the rebuilt projection.
pub(crate) fn refresh_conversation_projection(state: &mut TuiState) {
    if let Some(session) = state.session_snapshot.clone() {
        state.conversation = Some(conversation_projection(state, &session));
    } else {
        let mut conversation = crate::view::conversation::ConversationModel::default();
        attach_transient_rows(state, &mut conversation, None);
        state.conversation = Some(conversation);
    }
}

/// Adds ordered live rows to a durable or temporary conversation projection.
///
/// - `state`: Presentation state containing bounded transient observations.
/// - `conversation`: Projection receiving live rows.
/// - `session`: Authoritative snapshot used to suppress already durable output.
pub(crate) fn attach_transient_rows(
    state: &TuiState,
    conversation: &mut crate::view::conversation::ConversationModel,
    session: Option<&scriberig_session::Session>,
) {
    let mut transient_rows = Vec::new();
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
        transient_rows.push((
            activity.sequence,
            TransientConversationRow::Activity {
                branch,
                key: activity.key,
                label: format!("Live {:?}: {}", activity.activity_kind, summary),
            },
        ));
    }
    for output in &state.transient_agent_outputs {
        let Some(branch) = output.branch else {
            continue;
        };
        if session.is_some_and(|session| output_is_durable(session, output)) {
            continue;
        }
        transient_rows.push((
            output.sequence,
            TransientConversationRow::Output {
                branch,
                operation: output.operation.clone(),
                text: output.text.clone(),
            },
        ));
    }
    transient_rows.sort_by_key(|(sequence, _)| *sequence);
    for (_, row) in transient_rows {
        match row {
            TransientConversationRow::Activity { branch, key, label } => {
                conversation.insert_transient_activity(branch, key, label);
            }
            TransientConversationRow::Output {
                branch,
                operation,
                text,
            } => conversation.insert_transient_output(branch, &operation, &text),
        }
    }
}

/// Reports whether a complete transient output now exists in durable session data.
///
/// - `session`: Authoritative session snapshot.
/// - `output`: Operation-scoped transient output to compare.
///
/// Whether the latest assistant output on the associated branch matches exactly.
/// # Returns
/// Whether the observed output is now durably represented.
pub(crate) fn output_is_durable(
    session: &scriberig_session::Session,
    output: &super::TransientAgentOutput,
) -> bool {
    let Some(branch) = output.branch else {
        return false;
    };
    output.complete
        && session
            .branches
            .iter()
            .find(|candidate| candidate.id == branch)
            .and_then(|candidate| {
                candidate
                    .turns
                    .iter()
                    .rev()
                    .find_map(|turn| turn.agent.as_ref())
            })
            .is_some_and(|agent| crate::inspector::sanitize(&agent.text) == output.text)
}
