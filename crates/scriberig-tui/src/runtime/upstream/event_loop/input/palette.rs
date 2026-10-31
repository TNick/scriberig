//! Routes upstream palette choices into `ScribeRig` action reducers.

use std::io;
use std::sync::Arc;

use rpi_tui::tui::{OverlayOptions, SizeValue};
use rpi_tui::{
    Component, Editor, EditorOptions, EditorStyle, Focusable, SearchableSelectList, SelectItem,
    SelectList, TUI,
};

use crate::runtime::queue_effects;
use crate::service::TuiService;

use super::super::super::selector_factory::open_palette_selector;
use super::super::super::{ComposerInput, ControlKind};
use super::super::focus::restore_overlay_focus;
use super::InputContext;

/// Opens the upstream searchable command selector.
///
/// # Parameters
/// - `context`: Shared TUI state and upstream components used by the selector.
///
/// # Returns
/// Success after opening the upstream selector or leaving it closed.
///
/// # Errors
/// Returns an error when shared presentation state is poisoned.
pub(super) fn open<S: TuiService + Sync>(context: &mut InputContext<'_, '_, S>) -> io::Result<()> {
    let mut state = context
        .state
        .lock()
        .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
    crate::state::reduce(&mut state, crate::action::Action::PaletteOpened);
    drop(state);
    let previous_focus = context.screen.get_focus();
    let commands = context.service.plugin_commands();
    if open_palette_selector(
        context.screen,
        context.shared_state,
        context.input_sender,
        &commands,
    ) {
        if context.focus_before_overlay.is_none() {
            *context.focus_before_overlay = previous_focus;
        }
        context.editor.set_focused(false);
    }
    Ok(())
}

/// Applies a stable action selected from the upstream command palette.
///
/// # Parameters
/// - `context`: Shared state, components, and event channels for the active UI.
/// - `item_id`: Stable action or model-control identifier.
///
/// # Returns
/// `true` when the caller should skip the normal loop tail.
///
/// # Errors
/// Returns an error when shared presentation state is poisoned.
pub(super) fn select<S: TuiService + Sync>(
    context: &mut InputContext<'_, '_, S>,
    item_id: &str,
) -> io::Result<bool> {
    restore_overlay_focus(context.screen, context.editor, context.focus_before_overlay);
    if let Some(command) = item_id.strip_prefix("plugin-command:") {
        let response = context.service.invoke_plugin_command(command, "");
        present_plugin_response(context, command, "", response)?;
        return Ok(true);
    }
    let routed_input = match item_id {
        "model-picker" => Some(ComposerInput::OpenControl(ControlKind::Model)),
        "thinking-picker" => Some(ComposerInput::OpenControl(ControlKind::Thinking)),
        "theme-picker" => Some(ComposerInput::OpenThemeSelector),
        _ => None,
    };
    if let Some(input) = routed_input {
        if let Ok(mut state) = context.state.lock() {
            state.overlay = None;
        }
        let _ = context.input_sender.send(input);
        return Ok(true);
    }
    let mut state = context
        .state
        .lock()
        .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
    let effects = crate::state::activate_palette_item(&mut state, item_id);
    queue_effects(context.work_sender, context.pending_effects, effects);
    Ok(false)
}

/// Presents a structured plugin command result as safe TUI status text.
///
/// - `state`: Shared presentation state receiving the result.
/// - `response`: Result returned by the authorized upstream worker.
///
/// # Returns
/// Success after updating the status message.
///
/// # Errors
/// Returns an error when presentation state is poisoned.
pub(super) fn show_plugin_response(
    state: &std::sync::Arc<std::sync::Mutex<crate::state::TuiState>>,
    response: Result<serde_json::Value, String>,
) -> io::Result<()> {
    let message = match response {
        Ok(value) if value.get("kind").and_then(serde_json::Value::as_str) == Some("message") => {
            value
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Plugin command completed.")
                .to_owned()
        }
        Ok(_) => "Plugin command returned an unsupported interactive response.".to_owned(),
        Err(error) => format!("Plugin command failed: {error}"),
    };
    let mut state = state
        .lock()
        .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
    state.overlay = None;
    state.shell_message = crate::inspector::sanitize_external_text(&message);
    Ok(())
}

/// Presents one upstream plugin command result using upstream TUI controls.
///
/// - `context`: Active TUI components and authorized application service.
/// - `command`: Exact authorized plugin command identity.
/// - `arguments`: Original command arguments retained for UI continuations.
/// - `response`: Structured response produced by the isolated worker.
///
/// # Returns
/// Success after displaying a message or opening the requested interaction.
///
/// # Errors
/// Returns an error when presentation state is poisoned.
pub(super) fn present_plugin_response<S: TuiService + Sync>(
    context: &mut InputContext<'_, '_, S>,
    command: &str,
    arguments: &str,
    response: Result<serde_json::Value, String>,
) -> io::Result<()> {
    let Ok(value) = response else {
        return show_plugin_response(context.state, response);
    };
    match value.get("kind").and_then(serde_json::Value::as_str) {
        Some("message") => show_plugin_response(context.state, Ok(value)),
        Some("selector") => {
            present_plugin_selector(context, command, arguments, &value);
            Ok(())
        }
        Some("input" | "editor") => {
            present_plugin_editor(context, command, arguments, &value);
            Ok(())
        }
        _ => show_plugin_response(
            context.state,
            Err("Plugin command returned an unsupported UI response.".to_owned()),
        ),
    }
}

/// Converts one selected upstream selector item into its command action payload.
///
/// - `option`: Exact plugin-provided item selected by the user.
///
/// # Returns
/// A structured `value` action, preserving the complete item when no value is declared.
fn plugin_option_action(option: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "value": option.get("value").cloned().unwrap_or_else(|| option.clone())
    })
}

/// Opens an upstream selector for one plugin command response.
///
/// # Parameters
/// - `context`: Active TUI components and authorized application service.
/// - `command`: Exact authorized plugin command identity.
/// - `arguments`: Original command arguments retained for UI continuations.
/// - `value`: Structured plugin selector response.
///
fn present_plugin_selector<S: TuiService + Sync>(
    context: &mut InputContext<'_, '_, S>,
    command: &str,
    arguments: &str,
    value: &serde_json::Value,
) {
    let options = value
        .get("items")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    if options.is_empty() {
        let _ = show_plugin_response(
            context.state,
            Err("Plugin command returned an empty selector.".to_owned()),
        );
        return;
    }
    let items = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let label = option
                .get("title")
                .or_else(|| option.get("label"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Option");
            SelectItem::new(
                &index.to_string(),
                &crate::inspector::sanitize_external_text(label),
            )
        })
        .collect();
    let list = Arc::new(SelectList::new(items, 12));
    let sender = context.input_sender.clone();
    let command = command.to_owned();
    let arguments = arguments.to_owned();
    list.on_select(Arc::new(move |item| {
        if let Ok(index) = item.value.parse::<usize>()
            && let Some(option) = options.get(index)
        {
            let action = plugin_option_action(option);
            let _ = sender.send(ComposerInput::PluginCommandAction {
                command: command.clone(),
                arguments: arguments.clone(),
                action,
            });
        }
    }));
    let sender = context.input_sender.clone();
    list.on_cancel(Arc::new(move || {
        let _ = sender.send(ComposerInput::CancelSelection);
    }));
    let selector = Arc::new(SearchableSelectList::new(
        value.get("title").and_then(serde_json::Value::as_str),
        Some("Choose an option"),
        list,
    ));
    show_plugin_overlay(context, selector);
}

/// Opens an upstream editor for one plugin command response.
///
/// # Parameters
/// - `context`: Active TUI components and authorized application service.
/// - `command`: Exact authorized plugin command identity.
/// - `arguments`: Original command arguments retained for UI continuations.
/// - `value`: Structured plugin editor response.
fn present_plugin_editor<S: TuiService + Sync>(
    context: &mut InputContext<'_, '_, S>,
    command: &str,
    arguments: &str,
    value: &serde_json::Value,
) {
    let initial = value
        .get("initialText")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let placeholder = value
        .get("placeholder")
        .and_then(serde_json::Value::as_str)
        .or_else(|| value.get("title").and_then(serde_json::Value::as_str))
        .map(crate::inspector::sanitize_external_text);
    let editor = Arc::new(Editor::new(
        EditorOptions {
            placeholder,
            initial_text: Some(crate::inspector::sanitize_external_text(initial)),
            ..EditorOptions::default()
        },
        EditorStyle::default(),
        Arc::new(rpi_tui::get_keybindings().clone()),
    ));
    let sender = context.input_sender.clone();
    let command = command.to_owned();
    let arguments = arguments.to_owned();
    editor.on_submit(Arc::new(move |text| {
        let action = serde_json::json!({"value": crate::inspector::sanitize_external_text(text)});
        let _ = sender.send(ComposerInput::PluginCommandAction {
            command: command.clone(),
            arguments: arguments.clone(),
            action,
        });
    }));
    let sender = context.input_sender.clone();
    editor.on_cancel(Arc::new(move || {
        let _ = sender.send(ComposerInput::CancelSelection);
    }));
    show_plugin_overlay(context, editor);
}

/// Opens one upstream component as a modal overlay and retains the prior focus.
///
/// - `context`: Active terminal state and focus targets.
/// - `component`: Upstream component to display.
fn show_plugin_overlay<S>(context: &mut InputContext<'_, '_, S>, component: Arc<dyn Component>) {
    if context.focus_before_overlay.is_none() {
        *context.focus_before_overlay = context.screen.get_focus();
    }
    context.editor.set_focused(false);
    context.screen.set_focus(Some(Arc::clone(&component)));
    context.screen.show_overlay(
        component,
        Some(OverlayOptions {
            width: Some(SizeValue::Percent(0.72)),
            max_height: Some(SizeValue::Percent(0.72)),
            ..OverlayOptions::default()
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::show_plugin_response;
    use crate::state::TuiState;
    use std::sync::{Arc, Mutex};

    /// Keeps plugin-supplied terminal control bytes visible as safe text.
    #[test]
    fn plugin_command_message_is_sanitized_before_presentation() {
        let state = Arc::new(Mutex::new(TuiState::default()));
        show_plugin_response(
            &state,
            Ok(serde_json::json!({"kind": "message", "text": "done\u{1b}[2J"})),
        )
        .expect("plugin response is presented");
        let state = state.lock().expect("state lock");
        assert_eq!(state.shell_message, "done\\u{1b}[2J");
    }

    /// Rejects unknown plugin UI response kinds without invoking another path.
    #[test]
    fn unknown_plugin_command_ui_is_reported_without_execution() {
        let state = Arc::new(Mutex::new(TuiState::default()));
        show_plugin_response(
            &state,
            Ok(serde_json::json!({"kind": "widget", "items": []})),
        )
        .expect("plugin response is presented");
        assert!(
            state
                .lock()
                .expect("state lock")
                .shell_message
                .contains("unsupported")
        );
    }

    /// Preserves plugin selector values when submitting the action continuation.
    #[test]
    fn selected_plugin_option_becomes_structured_command_action() {
        assert_eq!(
            super::plugin_option_action(&serde_json::json!({"title": "Use", "value": "use"})),
            serde_json::json!({"value": "use"}),
        );
        let option = serde_json::json!({"title": "Use", "scope": "project"});
        assert_eq!(
            super::plugin_option_action(&option),
            serde_json::json!({"value": option}),
        );
    }
}
