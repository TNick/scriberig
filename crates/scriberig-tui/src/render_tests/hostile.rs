//! Covers hostile content renderer tests.

use super::*;

/// Exercises shared hostile strings through the integrated TUI overlay renderer.
#[test]
fn shared_hostile_corpus_is_inert_in_integrated_auxiliary_rendering() {
    let corpus: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/terminal_hostile_corpus.json"
    ))
    .expect("hostile corpus JSON");
    let labels = corpus
        .iter()
        .map(|entry| entry["value"].as_str().expect("corpus value").to_owned())
        .collect();
    let state = TuiState {
        overlay: Some(Overlay::SessionList(labels)),
        ..TuiState::default()
    };
    let mut terminal = Terminal::new(TestBackend::new(140, 40)).expect("terminal");

    terminal
        .draw(|frame| render(frame, &state))
        .expect("draw hostile corpus");
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();

    assert!(!rendered.chars().any(char::is_control));
    assert!(rendered.contains("\\u{1b}"), "{rendered}");
    assert!(rendered.contains("\\u{7}"), "{rendered}");
}

/// Verifies hostile corpus values stay inert in conversation and projection views.
#[test]
fn shared_hostile_corpus_is_inert_in_conversation_and_projection_views() {
    let corpus: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/terminal_hostile_corpus.json"
    ))
    .expect("hostile corpus JSON");
    let rows = corpus
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let entity = scriberig_core::EntityId::from_bytes(
                [u8::try_from(index % 256).expect("bounded entity byte"); 16],
            );
            crate::view::conversation::ConversationRow {
                source: crate::view::conversation::ConversationSource::Activity(entity),
                reply_anchor: None,
                depth: 0,
                label: entry["value"].as_str().expect("corpus value").to_owned(),
                kind: crate::view::conversation::ConversationRowKind::Activity,
            }
        })
        .collect();
    let state = TuiState {
        conversation: Some(crate::view::conversation::ConversationModel {
            rows,
            coordinator_cursor: None,
        }),
        ..TuiState::default()
    };
    let mut terminal = Terminal::new(TestBackend::new(140, 40)).expect("terminal");
    terminal
        .draw(|frame| render(frame, &state))
        .expect("draw hostile conversation");
    assert_buffer_has_no_terminal_controls(terminal.backend());

    let operation = scriberig_core::OperationId::new("hostile_fixture").expect("operation id");
    let outputs = corpus
        .iter()
        .enumerate()
        .map(|(sequence, entry)| crate::state::TransientAgentOutput {
            operation: operation.clone(),
            sequence: sequence as u64,
            branch: None,
            text: entry["value"].as_str().expect("corpus value").to_owned(),
            complete: true,
        })
        .collect();
    let session_state = TuiState {
        primary_view: crate::selection::PrimaryView::Session,
        session: crate::inspector::LoadState::Ready(crate::view::session::SessionModel::default()),
        transient_agent_outputs: outputs,
        session_composer: Some(crate::state::SessionComposerState {
            session: scriberig_core::EntityId::from_bytes([132; 16]),
            branch: None,
            text: String::new(),
            pending_prompt: None,
            error: Some(
                corpus[0]["value"]
                    .as_str()
                    .expect("corpus value")
                    .to_owned(),
            ),
        }),
        ..TuiState::default()
    };
    terminal
        .draw(|frame| render(frame, &session_state))
        .expect("draw hostile tool results and validation error");
    assert_buffer_has_no_terminal_controls(terminal.backend());

    let mut projection_state = TuiState {
        primary_view: crate::selection::PrimaryView::Projection,
        projection: crate::inspector::LoadState::Ready(crate::view::projection::ProjectionModel {
            projection: None,
            source_entity: scriberig_core::EntityId::from_bytes([130; 16]),
            source_path: None,
            source_revision: None,
            style: "plain".to_owned(),
            renderer: "fixture".to_owned(),
            content: corpus
                .iter()
                .filter_map(|entry| entry["value"].as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            synchronization: None,
            base_snapshot: scriberig_core::SnapshotId::new([131; 32]),
        }),
        ..TuiState::default()
    };
    projection_state.conversation = None;
    terminal
        .draw(|frame| render(frame, &projection_state))
        .expect("draw hostile projection");
    assert_buffer_has_no_terminal_controls(terminal.backend());
}
