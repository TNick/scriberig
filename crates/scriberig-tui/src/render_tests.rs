//! Verifies the deterministic TUI renderer at representative sizes.

use ratatui::{Terminal, backend::TestBackend};

use super::render;
use crate::state::{Focus, TuiState};

/// Renders representative sizes without touching a real terminal.
#[test]
fn renders_all_required_sizes() {
    for (width, height) in [(140, 40), (100, 30), (72, 24), (10, 3)] {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame, &TuiState::default()))
            .expect("draw");
    }
}

/// Verifies a refreshed long session selects its latest row inside the viewport.
#[test]
fn primary_session_scroll_keeps_latest_cursor_visible() {
    let state = TuiState {
        primary_view: crate::selection::PrimaryView::Session,
        focus: Focus::Primary,
        cursor: 39,
        ..TuiState::default()
    };

    assert_eq!(super::primary_scroll(&state, 40), (3, 0));
}

/// Renders retained live activity according to the local reasoning control.
#[test]
fn reasoning_visibility_rerenders_cached_live_activity_without_execution() {
    let mut state = TuiState {
        primary_view: crate::selection::PrimaryView::Session,
        session: crate::inspector::LoadState::Ready(crate::view::session::SessionModel::default()),
        transient_agent_activities: vec![crate::state::TransientAgentActivity {
            branch: "branch-1".to_owned(),
            activity_kind: scriberig_session::AgentActivityKind::ReasoningSummary,
            state: scriberig_session::AgentActivityState::Completed,
            summary: "Compare the selected revisions.".to_owned(),
        }],
        ..TuiState::default()
    };

    assert!(
        !super::render_primary::primary_text(&state).contains("Compare the selected revisions.")
    );
    state.reasoning_visibility = crate::state::ReasoningVisibility::Shown;
    assert!(
        super::render_primary::primary_text(&state).contains("Compare the selected revisions.")
    );
    assert_eq!(state.transient_agent_activities.len(), 1);
}

/// Renders durable model and thinking selection provenance after reload.
#[test]
fn session_activity_shows_recovered_execution_selection_history() {
    let state = TuiState {
        primary_view: crate::selection::PrimaryView::Session,
        session: crate::inspector::LoadState::Ready(crate::view::session::SessionModel {
            rows: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection {
                revision: 2,
                model_profile: Some("profile-b".to_owned()),
                model: Some("model-b".to_owned()),
                thinking_level: Some("careful".to_owned()),
                history: vec![scriberig_session::SessionExecutionChange {
                    revision: 2,
                    kind: scriberig_session::AgentActivityKind::ThinkingLevelChanged,
                    summary: "Thinking level selected: careful".to_owned(),
                    model: Some("model-b".to_owned()),
                    thinking_level: Some("careful".to_owned()),
                }],
            },
            tool_invocation_details: std::collections::BTreeMap::default(),
        }),
        ..TuiState::default()
    };

    let output = super::render_session::session_text(&state, 100).to_string();

    assert!(output.contains("Thinking selection · revision 2"));
    assert!(output.contains("Thinking level selected: careful"));
}

/// Verifies nested durable activity uses visible terminal depth rails.
#[test]
fn session_activity_uses_non_color_hierarchy_rails() {
    let entity = scriberig_core::EntityId::from_bytes([61; 16]);
    let mut state = TuiState {
        primary_view: crate::selection::PrimaryView::Session,
        session: crate::inspector::LoadState::Ready(crate::view::session::SessionModel {
            rows: vec![
                crate::view::session::SessionRow {
                    reference: entity,
                    depth: 0,
                    label: "Session".to_owned(),
                    kind: crate::view::session::SessionRowKind::Session,
                    candidate: None,
                    snapshot: None,
                    transient: false,
                },
                crate::view::session::SessionRow {
                    reference: entity,
                    depth: 1,
                    label: "Model: model-a".to_owned(),
                    kind: crate::view::session::SessionRowKind::ModelInvocation,
                    candidate: None,
                    snapshot: None,
                    transient: true,
                },
                crate::view::session::SessionRow {
                    reference: entity,
                    depth: 2,
                    label: "Tool: inspect".to_owned(),
                    kind: crate::view::session::SessionRowKind::ToolInvocation,
                    candidate: None,
                    snapshot: None,
                    transient: true,
                },
                crate::view::session::SessionRow {
                    reference: entity,
                    depth: 3,
                    label: "Tool result: completed".to_owned(),
                    kind: crate::view::session::SessionRowKind::ToolResult,
                    candidate: None,
                    snapshot: None,
                    transient: true,
                },
                crate::view::session::SessionRow {
                    reference: entity,
                    depth: 8,
                    label: "Deep nested activity".to_owned(),
                    kind: crate::view::session::SessionRowKind::ToolResult,
                    candidate: None,
                    snapshot: None,
                    transient: true,
                },
            ],
            execution: scriberig_session::SessionExecutionSelection::default(),
            tool_invocation_details: [(
                entity,
                "{\n  \"filter\": {\n    \"path\": \"src/lib.rs\"\n  }\n}".to_owned(),
            )]
            .into_iter()
            .collect(),
        }),
        ..TuiState::default()
    };

    let text = super::render_primary::primary_text(&state);

    assert_activity_rails(&state, &text);
    assert_activity_detail_modes(&mut state, entity, &text);
}

/// Verifies text and styling preserve visible non-color hierarchy rails.
///
/// # Parameters
/// - `state`: Cached session presentation to render.
/// - `text`: Plain text projection of the session rows.
fn assert_activity_rails(state: &TuiState, text: &str) {
    assert!(text.contains("▏ ~ Model: model-a"));
    assert!(text.contains("▏ ▏ ~ Tool: inspect"));
    assert!(text.contains("--filter"));
    assert!(text.contains("Tool result: completed"));
    let styled = super::render_session::session_text(state, 80);
    let rails = &styled.lines[2].spans;
    assert_eq!(rails[1].style.fg, Some(ratatui::style::Color::Cyan));
    assert_eq!(rails[2].style.fg, Some(ratatui::style::Color::Green));
    assert_eq!(rails[1].content, "▏ ");
    assert_eq!(rails[2].content, "▏ ");
    let deep_rails = &styled.lines[4].spans;
    assert_eq!(deep_rails[1].style.fg, Some(ratatui::style::Color::Cyan));
    assert_eq!(
        deep_rails[6].style.fg,
        Some(ratatui::style::Color::LightCyan)
    );
    assert_eq!(deep_rails[7].style.fg, Some(ratatui::style::Color::Cyan));
    assert_eq!(deep_rails[8].style.fg, Some(ratatui::style::Color::Green));
    let narrow = super::render_session::session_text(state, 14);
    let narrow_row = narrow.lines[2]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();
    assert!(narrow_row.contains("…"));
    assert!(narrow_row.contains("Tool: inspect"));
}

/// Verifies opt-in prompt, tool, and result details render from cached state.
///
/// # Parameters
/// - `state`: Cached session presentation to update with safe request and result details.
/// - `entity`: Stable session activity identity used for the cached detail.
/// - `text`: Initial rendering before request details are enabled.
fn assert_activity_detail_modes(
    state: &mut TuiState,
    entity: scriberig_core::EntityId,
    text: &str,
) {
    state.model_request_details.insert(
        entity,
        scriberig_protocol::ModelRequestDetailResponse {
            invocation: entity.to_string(),
            model: "model-a".to_owned(),
            thinking_level: Some("careful".to_owned()),
            messages: vec![scriberig_protocol::ModelRequestMessageDetail {
                role: "user".to_owned(),
                content: "Inspect the safe fixture.".to_owned(),
            }],
            tools: vec![scriberig_protocol::ModelRequestToolDetail {
                name: "inspect".to_owned(),
                description: "Read safe metadata".to_owned(),
                parameters: Some("{\"type\":\"object\"}".to_owned()),
            }],
            truncated: false,
        },
    );
    assert!(!text.contains("Model request"));
    state.prompt_visibility = crate::state::PromptVisibility::Shown;
    let prompt = super::render_primary::primary_text(state);
    assert!(prompt.contains("Model request"));
    assert!(prompt.contains("Inspect the safe fixture."));
    assert!(prompt.contains("Read safe metadata"));
    state.tool_result_details.insert(
        entity,
        scriberig_protocol::ToolResultDetailResponse {
            result: entity.to_string(),
            detail: serde_json::json!({"summary": "safe expanded result"}),
            truncated: false,
        },
    );
    state.tool_output_visibility = crate::state::ToolOutputVisibility::Full;
    assert!(super::render_primary::primary_text(state).contains("safe expanded result"));
    state.tool_invocation_visibility = crate::state::ToolInvocationVisibility::Detailed;
    let detailed = super::render_primary::primary_text(state);
    assert!(detailed.contains("Arguments:"));
    assert!(detailed.contains("    \"path\": \"src/lib.rs\""));
    state.tool_output_visibility = crate::state::ToolOutputVisibility::Hidden;
    assert!(super::render_primary::primary_text(state).contains("Tool result hidden"));
}
