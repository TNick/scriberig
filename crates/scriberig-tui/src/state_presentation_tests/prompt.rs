//! Covers one focused group of TUI regression tests.

use super::*;

/// Verifies prompt visibility loads captured requests without execution effects.
#[test]
fn prompt_visibility_requests_missing_model_details() {
    let session = scriberig_core::EntityId::from_bytes([81; 16]);
    let branch = scriberig_core::EntityId::from_bytes([82; 16]);
    let invocation = scriberig_core::EntityId::from_bytes([83; 16]);
    let mut state = TuiState {
        session: LoadState::Ready(crate::view::session::SessionModel {
            rows: vec![
                crate::view::session::SessionRow {
                    reference: session,
                    depth: 0,
                    label: "Session".to_owned(),
                    kind: crate::view::session::SessionRowKind::Session,
                    candidate: None,
                    snapshot: None,
                    transient: false,
                },
                crate::view::session::SessionRow {
                    reference: branch,
                    depth: 1,
                    label: "main".to_owned(),
                    kind: crate::view::session::SessionRowKind::Branch,
                    candidate: None,
                    snapshot: None,
                    transient: true,
                },
                crate::view::session::SessionRow {
                    reference: invocation,
                    depth: 2,
                    label: "Model: model-a".to_owned(),
                    kind: crate::view::session::SessionRowKind::ModelInvocation,
                    candidate: None,
                    snapshot: None,
                    transient: true,
                },
            ],
            execution: scriberig_session::SessionExecutionSelection::default(),
            tool_invocation_details: std::collections::BTreeMap::default(),
        }),
        overlay: Some(crate::overlay::Overlay::Palette(
            crate::overlay::palette::PaletteState {
                local_actions: vec![crate::overlay::palette::PaletteItem {
                    id: "toggle-prompt".to_owned(),
                    label: "Toggle captured model request".to_owned(),
                    source: crate::overlay::palette::ActionSource::Local,
                }],
                semantic_actions: Vec::new(),
            },
        )),
        ..TuiState::default()
    };

    let effects = crate::state::activate_palette_item(&mut state, "toggle-prompt");

    assert_eq!(
        effects,
        vec![Effect::ReadModelRequestDetail {
            session,
            branch,
            invocation,
        }]
    );
    assert_eq!(
        state.prompt_visibility,
        crate::state::PromptVisibility::Shown
    );

    let effects =
        crate::state::state_presentation::activate_palette_control(&mut state, "toggle-prompt")
            .expect("prompt toggle");

    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(
        state.prompt_visibility,
        crate::state::PromptVisibility::Hidden
    );

    let effects =
        crate::state::state_presentation::activate_palette_control(&mut state, "toggle-reasoning")
            .expect("reasoning toggle");

    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(
        state.reasoning_visibility,
        crate::state::ReasoningVisibility::Shown
    );
}
