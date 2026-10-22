//! Stores presentation-only state for the initial terminal shell.
#[path = "state_behavior.rs"]
mod behavior;

use std::collections::{BTreeSet, HashSet};

use crate::{
    action::Action,
    effect::Effect,
    explorer::ExplorerModel,
    inspector::{InspectorModel, LoadState},
    navigation::NavigationState,
    overlay::{Overlay, input::InputState},
    selection::EntitySelection,
};
use scriberig_app::RuntimeCapability;

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
    /// Omit invocation arguments while retaining the activity label.
    Hidden,
    /// Show trusted labels and bounded arguments on one compact line.
    #[default]
    CompactTruncated,
    /// Show the complete safe bounded argument projection.
    Full,
    /// Show one schema-redacted argument per line.
    StructuredParameters,
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
    /// Stable operation/category key used to update or clear this observation.
    pub key: u64,
    /// Local arrival order shared with streamed output presentation.
    pub sequence: u64,
    /// Runtime branch associated with the activity.
    pub branch: String,
    /// Provider-neutral activity category.
    pub activity_kind: scriberig_session::AgentActivityKind,
    /// Lifecycle state such as running or completed.
    pub state: scriberig_session::AgentActivityState,
    /// Safe bounded user-facing summary.
    pub summary: String,
    /// Monotonic arrival ordinal within the producing invocation.
    pub ordinal: u64,
}

/// Holds bounded, operation-scoped assistant output until durable refresh.
///
/// Public members
/// - `operation`: Shared operation producing the output.
/// - `branch`: Durable branch location when known.
/// - `text`: Bounded output currently shown while streaming.
/// - `complete`: Whether `text` is a complete shared output value.
///
/// Private members: None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransientAgentOutput {
    /// Shared operation producing the output.
    pub operation: scriberig_core::OperationId,
    /// Local arrival order shared with live activity presentation.
    pub sequence: u64,
    /// Durable branch location when known.
    pub branch: Option<scriberig_core::EntityId>,
    /// Bounded output currently shown while streaming.
    pub text: String,
    /// Whether `text` is a complete shared output value.
    pub complete: bool,
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

/// Stable conversation submission target independent of visible row position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversationTarget {
    /// The first message creates a session through the shared application API.
    NewSession,
    /// A message continues from an exact durable reply anchor.
    Existing {
        /// Existing session receiving the continuation.
        session: scriberig_core::EntityId,
        /// Exact validated location receiving the next user message.
        reply_anchor: scriberig_session::ContinuationAnchor,
    },
}

/// Persistent composer state owned by the conversation home.
///
/// # Public members
/// - `target`: Stable session or continuation target.
/// - `draft`: Current complete safe user draft.
/// - `pending_prompt`: Submitted text retained for failure recovery.
/// - `pending`: Whether submission awaits refresh.
/// - `error`: Safe submission error for display.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversationComposerState {
    /// Stable semantic target, separate from navigation and coordinator cursors.
    pub target: ConversationTarget,
    /// Multiline user-authored draft.
    pub draft: String,
    /// Submitted prompt retained while a new conversation is being accepted.
    pub pending_prompt: Option<String>,
    /// Whether an accepted submission is awaiting session refresh.
    pub pending: bool,
    /// Safe submission failure shown beside the composer.
    pub error: Option<String>,
}

impl ConversationComposerState {
    /// Appends input or removes the final character for an empty input.
    ///
    /// # Parameters
    /// - `text`: Text entered by the user.
    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            self.draft.pop();
        } else {
            self.draft.push_str(text);
        }
    }

    /// Appends a paste payload as data.
    ///
    /// Terminal escape and control sequences are escaped before insertion so a
    /// paste cannot drive the upstream renderer or editor.
    ///
    /// # Parameters
    /// - `text`: Pasted text, including any line breaks.
    pub fn paste_text(&mut self, text: &str) {
        self.draft.push_str(&sanitize_paste(text));
    }

    /// Clears the draft and resets the editor cursor.
    pub fn clear_draft(&mut self) {
        self.draft.clear();
    }
}

/// Escapes terminal control sequences in a paste payload.
///
/// Line breaks and tabs remain literal; other control characters are rendered
/// as their escaped form so they cannot act as terminal commands.
///
/// # Parameters
/// - `text`: Raw pasted text.
///
/// # Returns
/// Safe text suitable for insertion into a composer.
pub(crate) fn sanitize_paste(text: &str) -> String {
    let mut sanitized = String::with_capacity(text.len());
    for character in text.chars() {
        if character == '\n' || character == '\t' || !character.is_control() {
            sanitized.push(character);
        } else {
            sanitized.extend(character.escape_default());
        }
    }
    sanitized
}

/// Retains one stable selected node while its model assignment picker is open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanAssignmentTarget {
    /// Durable plan identity.
    pub plan: scriberig_core::EntityId,
    /// Durable plan-node identity.
    pub node: scriberig_core::EntityId,
    /// Plan revision observed before opening the picker.
    pub expected_revision: u64,
}

/// Runtime and presentation state for the initial TUI shell.
#[allow(clippy::struct_excessive_bools)] // Independent UI toggles are persisted presentation preferences.
#[derive(Clone, Debug, PartialEq)]
pub struct TuiState {
    /// Whether the event loop should terminate.
    pub should_quit: bool,
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
    /// Stable identity of the selected flat semantic-view row.
    pub semantic_row_cursor: Option<crate::selection::SemanticRowIdentity>,
    /// Stable plan/node navigation, independent of the conversation and coordinator cursors.
    pub plan_cursor: Option<crate::selection::PlanSelection>,
    /// Exact project scope available to the first agent conversation.
    pub agent_selection: Option<scriberig_app::AgentSelection>,
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
    /// Stable selected session row independent of its flattened position.
    pub session_cursor: Option<crate::view::session::SessionRowIdentity>,
    /// Conversation projection rebuilt from the latest durable session snapshot.
    pub conversation: Option<crate::view::conversation::ConversationModel>,
    /// Latest shared session snapshot used to rebuild local projections.
    pub session_snapshot: Option<scriberig_session::Session>,
    /// Conversation composer targeting an exact semantic location.
    pub conversation_composer: ConversationComposerState,
    /// Stable conversation cursor independent of rendered row index.
    pub conversation_cursor: Option<crate::view::conversation::ConversationSource>,
    /// Whether the user is browsing rows instead of editing the draft.
    pub conversation_browsing: bool,
    /// Durable node currently followed by the coordinator cursor.
    pub coordinator_cursor: Option<(scriberig_core::EntityId, scriberig_core::EntityId)>,
    /// Whether the conversation should follow the plan coordinator.
    pub follow_coordinator: bool,
    /// Client-local branch expansion identities.
    pub expanded_branches: BTreeSet<scriberig_core::EntityId>,
    /// Credential-safe model library available to the interactive picker.
    pub model_library: LoadState<scriberig_protocol::ModelLibraryResponse>,
    /// Whether a pending model-library read was opened from thinking controls.
    /// Selected plan node awaiting an assignment choice.
    pub plan_assignment_target: Option<PlanAssignmentTarget>,
    /// Branch to reveal after the matching agent submission finishes.
    pub session_branch_focus: Option<scriberig_core::EntityId>,
    /// Operation identities with an authoritative refresh already requested.
    pub pending_operation_refresh: Vec<scriberig_core::OperationId>,
    /// Bounded streamed assistant output keyed by shared operation identity.
    pub transient_agent_outputs: Vec<TransientAgentOutput>,
    /// Next local sequence for transient rows received from the shared stream.
    pub next_transient_sequence: u64,
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

pub(crate) use behavior::{
    conversation_projection, output_is_durable, refresh_conversation_projection,
};
#[path = "state_actions.rs"]
mod state_actions;
#[path = "state_cursor_reducer.rs"]
mod state_cursor_reducer;
#[path = "state_model_selection.rs"]
mod state_model_selection;
#[path = "state_presentation.rs"]
mod state_presentation;
#[path = "state_reducer.rs"]
mod state_reducer;
#[path = "state_reducer_tail.rs"]
mod state_reducer_tail;
pub(crate) use state_actions::activate_palette_item;
use state_actions::reduce_application_event;
pub(crate) use state_model_selection::{select_model, select_thinking_level};
pub(super) use state_presentation::activate_palette_control;
pub use state_reducer::reduce;

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
