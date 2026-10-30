//! Reduces shared personal source-style preference operations.

use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Handles personal source-style preference requests and exact results.
///
/// # Parameters
/// - `state`: TUI state receiving the setting lifecycle.
/// - `action`: Preference request, result, or failure to reduce.
///
/// # Returns
/// Shared preference effects and a projection refresh after successful updates.
pub(super) fn reduce(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::ProjectionStylePreferenceRequested(_)
        | Action::ProjectionStylePreferenceSet { .. }
        | Action::ProjectionStylePreferenceFailed { .. } => reduce_preference(state, action),
        Action::ProjectionStyleResolveRequested(_)
        | Action::ProjectionStyleResolved { .. }
        | Action::ProjectionStyleResolveFailed { .. } => reduce_resolution(state, action),
        _ => None,
    }
}

/// Reduces personal projection-style preference changes.
///
/// # Parameters
/// - `state`: TUI state receiving the preference lifecycle.
/// - `action`: Preference request, result, or failure.
///
/// # Returns
/// A shared style operation effect, or no-op for stale results.
fn reduce_preference(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::ProjectionStylePreferenceRequested(request) => {
            if request.projection_kind != "rust"
                || request.scope != scriberig_protocol::ProjectionStyleScope::User
            {
                state.conversation_composer.error = Some(
                    "The TUI source-style command accepts only the personal Rust style.".to_owned(),
                );
                return Some(vec![Effect::None]);
            }
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            state.projection_style_preference = LoadState::Loading { request_id };
            Some(vec![Effect::SetProjectionStylePreference {
                request_id,
                request: request.clone(),
            }])
        }
        Action::ProjectionStylePreferenceSet {
            request_id,
            setting,
        } => {
            if !matches!(state.projection_style_preference, LoadState::Loading { request_id: current } if current == *request_id)
            {
                return Some(vec![Effect::None]);
            }
            state.projection_style_preference = LoadState::Ready(setting.clone());
            state.shell_message = if setting.style.is_some() {
                "Personal Rust source style updated; refreshing the projection.".to_owned()
            } else {
                "Personal Rust source style cleared; refreshing the projection.".to_owned()
            };
            if state
                .selection
                .is_some_and(|selection| selection.snapshot.is_some())
            {
                Some(crate::state::reduce(state, Action::ProjectionOpened))
            } else {
                state.shell_message = state
                    .shell_message
                    .replace("; refreshing the projection.", ".");
                Some(vec![Effect::None])
            }
        }
        Action::ProjectionStylePreferenceFailed {
            request_id,
            category,
            message,
        } => {
            if matches!(state.projection_style_preference, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.projection_style_preference = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            Some(vec![Effect::None])
        }
        _ => None,
    }
}

/// Reduces effective projection-style resolution requests and results.
///
/// # Parameters
/// - `state`: TUI state receiving the resolution lifecycle.
/// - `action`: Resolution request, result, or failure.
///
/// # Returns
/// A shared resolution effect, or no-op for stale results.
fn reduce_resolution(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::ProjectionStyleResolveRequested(request) => {
            if request.projection_kind != "rust"
                || request.purpose != scriberig_protocol::ProjectionStylePurpose::InteractiveView
            {
                state.conversation_composer.error = Some(
                    "The TUI can inspect only the effective interactive Rust source style."
                        .to_owned(),
                );
                return Some(vec![Effect::None]);
            }
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            state.projection_style_resolution = LoadState::Loading { request_id };
            Some(vec![Effect::ResolveProjectionStyle {
                request_id,
                request: request.clone(),
            }])
        }
        Action::ProjectionStyleResolved {
            request_id,
            resolution,
        } => {
            if !matches!(state.projection_style_resolution, LoadState::Loading { request_id: current } if current == *request_id)
            {
                return Some(vec![Effect::None]);
            }
            state.projection_style_resolution = LoadState::Ready(resolution.clone());
            state.shell_message = crate::view::projection_style::summary(resolution);
            Some(vec![Effect::None])
        }
        Action::ProjectionStyleResolveFailed {
            request_id,
            category,
            message,
        } => {
            if matches!(state.projection_style_resolution, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.projection_style_resolution = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
                state.shell_message = format!("Style inspection failed ({category}): {message}");
            }
            Some(vec![Effect::None])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::reduce;
    use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

    /// Keeps user style preference writes out of selection and model lifecycle state.
    #[test]
    fn preference_update_does_not_move_cursors_or_start_model_work() {
        let mut state = TuiState::default();
        let terminal_theme_revision = rpi_tui::theme_revision();
        state.cursor = 7;
        let cursor_entity = scriberig_core::EntityId::from_bytes([44; 16]);
        state.plan_cursor = Some(crate::selection::PlanSelection {
            plan: cursor_entity,
            node: Some(cursor_entity),
        });
        state.session_cursor = Some(crate::view::session::SessionRowIdentity {
            reference: cursor_entity,
            kind: crate::view::session::SessionRowKind::Branch,
        });
        state.conversation_cursor = Some(crate::view::conversation::ConversationSource::Branch(
            cursor_entity,
        ));
        state.coordinator_cursor = Some((cursor_entity, cursor_entity));
        let plan_cursor = state.plan_cursor;
        let session_cursor = state.session_cursor;
        let conversation_cursor = state.conversation_cursor;
        let coordinator_cursor = state.coordinator_cursor;
        let conversation_target = state.conversation_composer.target;
        let selection = state.selection;
        let request = scriberig_protocol::ProjectionStyleSettingRequest {
            scope: scriberig_protocol::ProjectionStyleScope::User,
            projection_kind: "rust".to_owned(),
            expected_revision: None,
            style: Some(b"indent=tabs,indent-width=2,blank-lines=0".to_vec()),
        };
        assert!(matches!(
            reduce(
                &mut state,
                &Action::ProjectionStylePreferenceRequested(request)
            ),
            Some(effects) if matches!(effects.as_slice(), [Effect::SetProjectionStylePreference { .. }])
        ));
        assert_eq!(state.cursor, 7);
        assert_eq!(state.plan_cursor, plan_cursor);
        assert_eq!(state.session_cursor, session_cursor);
        assert_eq!(state.conversation_cursor, conversation_cursor);
        assert_eq!(state.coordinator_cursor, coordinator_cursor);
        assert_eq!(state.conversation_composer.target, conversation_target);
        assert_eq!(state.selection, selection);
        assert!(matches!(
            state.projection_style_preference,
            LoadState::Loading { .. }
        ));
        assert_eq!(rpi_tui::theme_revision(), terminal_theme_revision);
    }

    /// Refreshes the selected projection without moving conversation state.
    #[test]
    fn successful_preference_update_refreshes_projection_only() {
        let mut state = TuiState::default();
        let terminal_theme_revision = rpi_tui::theme_revision();
        let snapshot = scriberig_core::SnapshotId::new([46; 32]);
        state.selection = Some(crate::selection::EntitySelection {
            entity: scriberig_core::EntityId::from_bytes([45; 16]),
            revision: None,
            snapshot: Some(snapshot),
        });
        state.cursor = 5;
        let cursor_entity = scriberig_core::EntityId::from_bytes([47; 16]);
        state.plan_cursor = Some(crate::selection::PlanSelection {
            plan: cursor_entity,
            node: Some(cursor_entity),
        });
        state.session_cursor = Some(crate::view::session::SessionRowIdentity {
            reference: cursor_entity,
            kind: crate::view::session::SessionRowKind::Turn,
        });
        state.conversation_cursor = Some(crate::view::conversation::ConversationSource::Activity(
            cursor_entity,
        ));
        state.coordinator_cursor = Some((cursor_entity, cursor_entity));
        let plan_cursor = state.plan_cursor;
        let session_cursor = state.session_cursor;
        let conversation_cursor = state.conversation_cursor;
        let coordinator_cursor = state.coordinator_cursor;
        let conversation_target = state.conversation_composer.target;
        state.projection_style_preference = LoadState::Loading { request_id: 9 };
        let effects = reduce(
            &mut state,
            &Action::ProjectionStylePreferenceSet {
                request_id: 9,
                setting: scriberig_protocol::ProjectionStyleSettingResponse {
                    style: Some(b"indent=tabs,indent-width=2,blank-lines=0".to_vec()),
                    revision: Some(4),
                },
            },
        )
        .expect("style result handled");
        assert!(matches!(effects.as_slice(), [Effect::Render { .. }]));
        assert_eq!(state.cursor, 5);
        assert_eq!(state.plan_cursor, plan_cursor);
        assert_eq!(state.session_cursor, session_cursor);
        assert_eq!(state.conversation_cursor, conversation_cursor);
        assert_eq!(state.coordinator_cursor, coordinator_cursor);
        assert_eq!(state.conversation_composer.target, conversation_target);
        assert!(matches!(state.projection, LoadState::Loading { .. }));
        assert_eq!(rpi_tui::theme_revision(), terminal_theme_revision);
    }

    /// Retains and summarizes resolver provenance and the exact fidelity classification.
    #[test]
    fn effective_style_resolution_is_visible_without_model_or_cursor_work() {
        let mut state = TuiState {
            cursor: 3,
            ..TuiState::default()
        };
        let request = scriberig_protocol::ProjectionStyleResolveRequest {
            projection_kind: "rust".to_owned(),
            project_id: "project-a".to_owned(),
            workspace_id: "project-a".to_owned(),
            purpose: scriberig_protocol::ProjectionStylePurpose::InteractiveView,
            view_id: Some("project-source:entity-a".to_owned()),
            explicit_personal: None,
        };
        let effects = reduce(
            &mut state,
            &Action::ProjectionStyleResolveRequested(request),
        )
        .expect("resolution request handled");
        let [Effect::ResolveProjectionStyle { request_id, .. }] = effects.as_slice() else {
            panic!("resolution uses the shared service effect: {effects:?}");
        };
        let response = scriberig_protocol::ProjectionStyleResolveResponse {
            purpose: scriberig_protocol::ProjectionStylePurpose::InteractiveView,
            reference: scriberig_protocol::ProjectionStyleReference::Inline,
            value: b"indent=spaces,indent-width=4".to_vec(),
            schema: Some(scriberig_protocol::ProjectionStyleSchemaReference {
                adapter_id: "rust-source".to_owned(),
                version: "1".to_owned(),
            }),
            applied: vec![scriberig_protocol::ProjectionStyleSource {
                kind: "user-preference".to_owned(),
                identity: "alice".to_owned(),
                revision: Some("r7".to_owned()),
                generation: None,
                value: b"indent=spaces,indent-width=4".to_vec(),
            }],
            fidelity: scriberig_protocol::ProjectionStyleFidelityReport {
                fidelity: scriberig_protocol::ProjectionStyleFidelity::LosslessEditable,
                omitted_categories: Vec::new(),
                explanation: Some("all modeled syntax is retained".to_owned()),
            },
        };
        let response_effects = reduce(
            &mut state,
            &Action::ProjectionStyleResolved {
                request_id: *request_id,
                resolution: response,
            },
        )
        .expect("resolution result handled");
        assert_eq!(response_effects, vec![Effect::None]);
        assert!(state.shell_message.contains("lossless editable"));
        assert!(state.shell_message.contains("user-preference:alice"));
        assert!(state.shell_message.contains("omitted: none"));
        assert_eq!(state.cursor, 3);
    }
}
