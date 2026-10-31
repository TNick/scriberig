//! Routes recognized conversation slash commands to local effects.

use super::*;

#[path = "commands/assignment_command.rs"]
mod assignment_command;
#[path = "commands/cad_command.rs"]
mod cad_command;
#[path = "commands/document_view.rs"]
mod document_view;
#[path = "commands/inspection_command.rs"]
mod inspection_command;
#[path = "commands/navigation_command.rs"]
mod navigation_command;
#[path = "commands/project_artifacts.rs"]
mod project_artifacts;
#[path = "commands/session_argument.rs"]
mod session_argument;
#[path = "commands/spreadsheet_views.rs"]
mod spreadsheet_views;

#[path = "commands/document_command.rs"]
mod document_command;
#[path = "commands/geo_command.rs"]
mod geo_command;
#[path = "commands/plan_command.rs"]
mod plan_command;
#[path = "commands/qgis_command.rs"]
mod qgis_command;
#[path = "commands/recovery.rs"]
mod recovery;
#[path = "commands/source_style_command.rs"]
mod source_style_command;
#[path = "commands/spreadsheet_command.rs"]
mod spreadsheet_command;

/// Routes supported console commands to shared picker and preference actions.
///
/// - `state`: Current TUI state used by the shared action reducers.
/// - `text`: Submitted composer text.
///
/// Effects for a recognized command, or `None` for an agent prompt.
/// # Returns
/// The operation result.
#[allow(clippy::too_many_lines)] // Slash routing remains one descriptor-driven command reducer.
pub(super) fn console_command(state: &mut TuiState, text: &str) -> Option<Vec<Effect>> {
    let command = text.trim();
    let name = command.split_whitespace().next().unwrap_or(command);
    let Some(descriptor) = crate::slash_commands::find(name) else {
        if command.starts_with('/') {
            state.conversation_composer.error =
                Some(format!("Unknown command: {name}. Use /help for commands."));
            return Some(vec![Effect::None]);
        }
        return None;
    };
    match command {
        "/model" | "/thinking" => Some(vec![Effect::None]),
        "/view reasoning" => super::super::state_presentation::activate_presentation_action(
            state,
            "toggle-reasoning",
        ),
        "/view prompt" => {
            super::super::state_presentation::activate_presentation_action(state, "toggle-prompt")
        }
        "/view tool-call" => super::super::state_presentation::activate_presentation_action(
            state,
            "toggle-tool-invocation",
        ),
        "/view tool-output" => super::super::state_presentation::activate_presentation_action(
            state,
            "toggle-tool-output",
        ),
        _ if descriptor.name == "/help" => {
            state.overlay = Some(crate::overlay::Overlay::Help);
            Some(vec![Effect::None])
        }
        _ if descriptor.name == "/new" => {
            Some(super::super::state_reducer_tail::new_conversation(state))
        }
        _ if descriptor.name == "/sessions" => {
            if command != "/sessions" {
                state.conversation_composer.error =
                    Some("Use /sessions without arguments.".to_owned());
                return Some(vec![Effect::None]);
            }
            state.overlay = None;
            Some(vec![Effect::ListSessions])
        }
        _ if descriptor.name == "/switch" => {
            match session_argument::parse_session_id(command, "/switch") {
                Ok(session) => {
                    state.overlay = None;
                    Some(vec![Effect::SwitchSession { session }])
                }
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    Some(vec![Effect::None])
                }
            }
        }
        _ if descriptor.name == "/remove-session" => {
            match session_argument::parse_session_id(command, "/remove-session") {
                Ok(session) => {
                    state.overlay = None;
                    Some(vec![Effect::RemoveSession { session }])
                }
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    Some(vec![Effect::None])
                }
            }
        }
        _ if descriptor.name == "/recover" => {
            state.recovery_message = None;
            let Some(session) = state.session_snapshot.as_ref() else {
                state.conversation_composer.error =
                    Some("Load a session before inspecting legacy choices.".to_owned());
                return Some(vec![Effect::None]);
            };
            match recovery::parse(command, session) {
                Ok(effect) => Some(vec![effect]),
                Err(message) => {
                    state.conversation_composer.error = Some(message);
                    Some(vec![Effect::None])
                }
            }
        }
        _ if descriptor.name == "/edit" => {
            Some(super::super::state_actions::begin_projection_edit(state))
        }
        _ if descriptor.name == "/session" => {
            if matches!(state.session, crate::inspector::LoadState::Ready(_)) {
                state.primary_view = crate::selection::PrimaryView::Session;
                Some(vec![Effect::None])
            } else {
                state.conversation_composer.error = Some("No session is loaded yet.".to_owned());
                Some(vec![Effect::None])
            }
        }
        _ if descriptor.name == "/explorer" => {
            let Some(selection) = state.selection.as_ref() else {
                state.conversation_composer.error =
                    Some("Select a semantic entity before opening Explorer.".to_owned());
                return Some(vec![Effect::None]);
            };
            let Some(snapshot) = selection.snapshot else {
                state.conversation_composer.error =
                    Some("Explorer requires an exact snapshot selection.".to_owned());
                return Some(vec![Effect::None]);
            };
            state.primary_view = crate::selection::PrimaryView::Explorer;
            state.cursor = 0;
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            Some(vec![Effect::ListExplorer {
                request_id,
                entity: selection.entity,
                snapshot,
                continuation: None,
            }])
        }
        _ if descriptor.name == "/plan" => Some(plan_command::plan_command(
            state,
            command.strip_prefix("/plan").unwrap_or_default().trim(),
        )),
        _ if descriptor.name == "/assign" => Some(assignment_command::assign(state)),
        _ if descriptor.name == "/operations" => Some(super::reduce(
            state,
            Action::ViewSelected(crate::selection::PrimaryView::Operations),
        )),
        _ if descriptor.name == "/project" => Some(project_artifacts::active_project(state)),
        _ if descriptor.name == "/artifacts" => Some(project_artifacts::artifacts(state, command)),
        _ if descriptor.name == "/artifact-members" => {
            Some(project_artifacts::members(state, command))
        }
        _ if descriptor.name == "/cad" => {
            let args = command.strip_prefix(name).unwrap_or_default().trim();
            match cad_command::read_action(state, args) {
                Ok(action) => Some(super::reduce(state, action)),
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    Some(vec![Effect::None])
                }
            }
        }
        _ if descriptor.name == "/cad-select" => {
            let args = command.strip_prefix(name).unwrap_or_default().trim();
            match cad_command::select_entity(args) {
                Ok(action) => Some(super::reduce(state, action)),
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    Some(vec![Effect::None])
                }
            }
        }
        _ if descriptor.name == "/cad-layer" => {
            let args = command.strip_prefix(name).unwrap_or_default().trim();
            match cad_command::layer_change(state, args) {
                Ok(request) => Some(super::reduce(
                    state,
                    Action::CadLayerChangeRequested(request),
                )),
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    Some(vec![Effect::None])
                }
            }
        }
        _ if descriptor.name == "/geo" => {
            let (project, snapshot) = match geo_command::scope(state) {
                Ok(scope) => scope,
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    return Some(vec![Effect::None]);
                }
            };
            let (path, offset) = match geo_command::catalog_arguments(
                command.strip_prefix(name).unwrap_or_default().trim(),
            ) {
                Ok(value) => value,
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    return Some(vec![Effect::None]);
                }
            };
            Some(super::reduce(
                state,
                Action::GeoCatalogRequested {
                    project,
                    path,
                    snapshot,
                    offset,
                },
            ))
        }
        _ if descriptor.name == "/qgis" => {
            let args = command.strip_prefix(name).unwrap_or_default().trim();
            match qgis_command::project_request(state, args) {
                Ok(action) => Some(super::reduce(state, action)),
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    Some(vec![Effect::None])
                }
            }
        }
        _ if descriptor.name == "/qgis-layer" => {
            let args = command.strip_prefix(name).unwrap_or_default().trim();
            match qgis_command::layer_request(state, args) {
                Ok(action) => Some(super::reduce(state, action)),
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    Some(vec![Effect::None])
                }
            }
        }
        _ if descriptor.name == "/qgis-rename" => {
            let args = command.strip_prefix(name).unwrap_or_default().trim();
            match qgis_command::rename_request(state, args) {
                Ok(action) => Some(super::reduce(state, action)),
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    Some(vec![Effect::None])
                }
            }
        }
        _ if descriptor.name == "/qgis-order" => {
            let args = command.strip_prefix(name).unwrap_or_default().trim();
            match qgis_command::reorder_request(state, args) {
                Ok(action) => Some(super::reduce(state, action)),
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    Some(vec![Effect::None])
                }
            }
        }
        _ if descriptor.name == "/geo-features" => {
            let (layer, offset) = match geo_command::feature_arguments(
                command.strip_prefix(name).unwrap_or_default().trim(),
            ) {
                Ok(value) => value,
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    return Some(vec![Effect::None]);
                }
            };
            Some(super::reduce(
                state,
                Action::GeoFeaturesRequested { layer, offset },
            ))
        }
        _ if descriptor.name == "/geo-query" => {
            let args = command.strip_prefix(name).unwrap_or_default().trim_start();
            match geo_command::query_request(state, args) {
                Ok(request) => Some(super::reduce(
                    state,
                    Action::GeoFeatureQueryRequested(request),
                )),
                Err(message) => {
                    state.conversation_composer.error = Some(message);
                    Some(vec![Effect::None])
                }
            }
        }
        _ if descriptor.name == "/geo-set" => {
            let args = command.strip_prefix(name).unwrap_or_default().trim_start();
            let edit = match geo_command::attribute_edit(state, args) {
                Ok(edit) => edit,
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    return Some(vec![Effect::None]);
                }
            };
            Some(super::reduce(
                state,
                Action::GeoAttributeEditRequested(edit),
            ))
        }
        _ if descriptor.name == "/geo-select" => {
            let entity = command
                .strip_prefix(name)
                .unwrap_or_default()
                .trim()
                .parse::<scriberig_core::EntityId>();
            if let Ok(feature) = entity {
                Some(super::reduce(state, Action::GeoFeatureSelected { feature }))
            } else {
                state.conversation_composer.error =
                    Some("Usage: /geo-select <feature-id>.".to_owned());
                Some(vec![Effect::None])
            }
        }
        _ if matches!(
            descriptor.name,
            "/spreadsheet" | "/spreadsheet-cell" | "/spreadsheet-set" | "/spreadsheet-formula"
        ) =>
        {
            spreadsheet_views::dispatch(state, command, name, descriptor.name)
        }
        _ if descriptor.name == "/geo-geometry" => {
            let args = command.strip_prefix(name).unwrap_or_default().trim_start();
            match geo_command::geometry_edit(state, args) {
                Ok(request) => Some(super::reduce(
                    state,
                    Action::GeoGeometryEditRequested(request),
                )),
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    Some(vec![Effect::None])
                }
            }
        }

        _ if descriptor.name == "/document" => Some(document_view::outline(state, command)),
        _ if descriptor.name == "/document-select" => {
            let argument = command.strip_prefix(name).unwrap_or_default().trim();
            let Ok(entity) = argument.parse::<scriberig_core::EntityId>() else {
                state.conversation_composer.error = Some(
                    "Usage: /document-select <semantic paragraph or table-cell id>.".to_owned(),
                );
                return Some(vec![Effect::None]);
            };
            Some(super::reduce(
                state,
                Action::DocumentPartSelected { entity },
            ))
        }
        _ if descriptor.name == "/document-replace" => {
            let args = command.strip_prefix(name).unwrap_or_default().trim_start();
            Some(document_command::replace_paragraph(state, args))
        }
        _ if descriptor.name == "/document-cell-replace" => {
            let args = command.strip_prefix(name).unwrap_or_default().trim_start();
            Some(document_command::replace_table_cell(state, args))
        }
        _ if descriptor.name == "/history" => Some(navigation_command::history(state)),
        _ if descriptor.name == "/diff" => Some(super::reduce(state, Action::DiffChooserOpened)),
        _ if descriptor.name == "/projection" => {
            Some(super::reduce(state, Action::ProjectionOpened))
        }
        _ if descriptor.name == "/source-style" => {
            Some(navigation_command::source_style(state, command, name))
        }
        _ if descriptor.name == "/changes" => {
            Some(super::super::state_actions::refresh_candidate_review(state))
        }
        _ if descriptor.name == "/find" && command.len() > name.len() => Some(super::reduce(
            state,
            Action::FindRequested(command[name.len()..].trim().to_owned()),
        )),
        _ if descriptor.name == "/inspect" => Some(inspection_command::inspect(state, command)),
        _ if descriptor.name == "/status" => Some(navigation_command::status(state)),
        _ => {
            state.conversation_composer.error = Some(format!(
                "{} is recognized but requires an active shared view or plan target.",
                descriptor.name
            ));
            Some(vec![Effect::None])
        }
    }
}
