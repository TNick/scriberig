//! Shared fixtures for the transcript viewport follow tests.

use std::sync::{Arc, Mutex};

use rpi_tui::layout::render_layout_frame;
use rpi_tui::{Component, Keybindings, ScrollView, Text, VStack};

use super::super::{SemanticViewPanel, arrange_shell_panes};
use crate::state::TuiState;
use scriberig_core::EntityId;

/// Text that must reach the painted frame when the transcript follows the newest row.
pub(super) const NEWEST_ROW: &str = "NEWEST continuation result";
/// Text identifying the fixed composer pane at the bottom of the shell.
pub(super) const COMPOSER_ROW: &str = "COMPOSER pane";
/// Painted terminal geometry used by the native PTY gates.
pub(super) const TERMINAL_ROWS: usize = 40;

/// Builds one entity whose bytes are distinct per slot and index.
///
/// - `slot`: Distinct high byte separating entity families.
/// - `index`: Position within that family.
///
/// # Returns
/// A deterministic session entity identity.
pub(super) fn entity(slot: u8, index: u32) -> EntityId {
    let mut bytes = [0_u8; 16];
    bytes[0] = 0xbb;
    bytes[1] = slot;
    bytes[12..].copy_from_slice(&index.to_be_bytes());
    EntityId::from_bytes(bytes)
}

/// Publishes the durable result the newest continuation returns.
///
/// - `session`: Session whose newest continuation branch completes.
pub(super) fn publish_newest_completion(session: &mut scriberig_session::Session) {
    let continuation = entity(0xa1, 0);
    let branch = session
        .branches
        .iter_mut()
        .find(|branch| branch.id == continuation)
        .expect("newest branch");
    branch.result = Some(scriberig_session::BranchResult {
        id: entity(0xa3, 0),
        branch: continuation,
        status: scriberig_session::BranchStatus::Completed,
        summary: Some(NEWEST_ROW.to_owned()),
        result_kind: Some("agent".to_owned()),
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
}

/// Appends one model invocation with its tool call and result to a branch.
///
/// - `branch`: Branch receiving the ordered activity triple.
/// - `index`: Position that makes every recorded identity distinct.
fn append_activity(branch: &mut scriberig_session::Branch, index: u32) {
    let invocation = entity(1, index);
    let tool = entity(2, index);
    let result = entity(3, index);
    branch
        .model_invocations
        .push(scriberig_session::ModelInvocation {
            id: invocation,
            branch: branch.id,
            ordinal: u64::from(index),
            agent_profile: None,
            context_fingerprint: None,
            provider_profile: None,
            provider_id: None,
            model: Some("default".to_owned()),
            model_profile: None,
            provider_model_id: None,
            thinking_level: None,
            request_fingerprint: None,
            cache: scriberig_session::CacheDisposition::Miss,
            output: None,
            request: None,
        });
    branch
        .tool_invocations
        .push(scriberig_session::ToolInvocation {
            id: tool,
            name: "project_manifest".to_owned(),
            input: Vec::new(),
            parent_model_invocation: Some(invocation),
            presentation_input: None,
            presentation_label: None,
            category: None,
        });
    branch.tool_results.push(scriberig_session::ToolResult {
        id: result,
        invocation: tool,
        output: Vec::new(),
        failed: false,
        presentation_preview: None,
    });
    branch
        .activity
        .push(scriberig_session::ExecutionActivity::ModelInvocation(
            invocation,
        ));
    branch
        .activity
        .push(scriberig_session::ExecutionActivity::ToolInvocation(tool));
    branch
        .activity
        .push(scriberig_session::ExecutionActivity::ToolResult(result));
}

/// Builds an onboarding session whose newest continuation ends below taller-than-screen
/// history, mirroring the native self-hosting conversation.
///
/// - `history`: Model invocations recorded on the completed onboarding branch.
/// - `published`: Whether the newest continuation already carries its durable result.
///
/// # Returns
/// The durable session containing the newest completed continuation.
pub(super) fn session_with_newest_continuation(
    history: u32,
    published: bool,
) -> scriberig_session::Session {
    let onboarding = entity(0xa0, 0);
    let continuation = entity(0xa1, 0);
    let mut root = scriberig_session::Branch::new(onboarding, "agent root").expect("root branch");
    root.turns.push(scriberig_session::Turn {
        id: entity(4, 0),
        ordinal: 0,
        human: Some(scriberig_session::HumanInput {
            text: "Onboard the current project.".to_owned(),
        }),
        agent: Some(scriberig_session::AgentOutput {
            text: "onboarded".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    root.activity
        .push(scriberig_session::ExecutionActivity::HumanInput(entity(
            4, 0,
        )));
    root.activity
        .push(scriberig_session::ExecutionActivity::AgentOutput(entity(
            4, 0,
        )));
    for index in 0..history {
        append_activity(&mut root, index);
    }
    root.status = scriberig_session::BranchStatus::Completed;
    let mut session = scriberig_session::Session {
        id: entity(0xa2, 0),
        branches: vec![root],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    session
        .add_continuation_branch(
            continuation,
            scriberig_session::ContinuationAnchor::BranchHead { branch: onboarding },
            "agent continuation",
            "add PHP ingestion",
            scriberig_session::ContinuationOrigin::HumanContinuation,
        )
        .expect("continuation branch");
    let newest = session
        .branches
        .iter_mut()
        .find(|branch| branch.id == continuation)
        .expect("newest branch");
    append_activity(&mut *newest, 100);
    newest.status = scriberig_session::BranchStatus::Completed;
    if published {
        publish_newest_completion(&mut session);
    }
    session
}

/// Builds presentation state selecting the newest continuation branch.
///
/// - `session`: Durable session to project into the conversation home.
/// - `continuation`: Newest branch identity.
///
/// # Returns
/// Conversation presentation state for the rendered shell.
pub(super) fn following_state(
    session: &scriberig_session::Session,
    continuation: EntityId,
) -> TuiState {
    TuiState {
        primary_view: crate::selection::PrimaryView::Conversation,
        session_snapshot: Some(session.clone()),
        conversation: Some(crate::state::conversation_projection(
            &TuiState::default(),
            session,
        )),
        conversation_cursor: Some(crate::view::conversation::ConversationSource::Branch(
            continuation,
        )),
        ..TuiState::default()
    }
}

/// Builds the production shell: status pane, transcript scroll view, and composer pane.
///
/// - `state`: Shared presentation state rendered by the semantic panel.
///
/// # Returns
/// The layout root and the transcript scroll view it contains.
pub(super) fn shell(state: Arc<Mutex<TuiState>>) -> (Arc<dyn Component>, Arc<ScrollView>) {
    let (sender, _receiver) = std::sync::mpsc::channel();
    let panel = Arc::new(SemanticViewPanel::new(
        state,
        sender,
        Arc::new(Keybindings::new()),
    ));
    let transcript = Arc::new(ScrollView::primary(panel));
    let root = Arc::new(VStack::new());
    arrange_shell_panes(
        &root,
        Arc::new(Text::new("status pane", 0, 0)),
        &transcript,
        Arc::new(Text::new(COMPOSER_ROW, 0, 0)),
    );
    (root, transcript)
}

/// Paints one frame of the shell at the native PTY terminal size.
///
/// - `root`: Shell layout root retained between frames.
///
/// # Returns
/// The painted rows.
pub(super) fn paint(root: &Arc<dyn Component>) -> Vec<String> {
    render_layout_frame(Arc::clone(root), 140, TERMINAL_ROWS).lines
}
