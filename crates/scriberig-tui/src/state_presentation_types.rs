//! Defines reusable presentation-state values and safe composer input helpers.

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
