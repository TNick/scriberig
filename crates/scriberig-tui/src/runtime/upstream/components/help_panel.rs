//! Renders and handles the active `ScribeRig` help overlay.

use std::any::Any;
use std::fmt::Write as _;
use std::sync::mpsc::Sender;

use rpi_tui::{Component, InputEvent, KeybindingId, Keybindings, Markdown};

use super::super::ComposerInput;

/// Displays upstream editor bindings and `ScribeRig` semantic controls.
///
/// # Public members
/// None.
///
/// # Private members
/// - `markdown`: Upstream Markdown renderer for generated help rows.
/// - `input_sender`: Channel for closing the upstream overlay.
pub(crate) struct HelpPanel {
    /// Upstream Markdown renderer for generated help rows.
    markdown: Markdown,
    /// Channel used to close the active help overlay.
    input_sender: Sender<ComposerInput>,
}

impl HelpPanel {
    /// Creates help content from active upstream editor bindings and controls.
    ///
    /// # Parameters
    /// - `input_sender`: Channel used to report a close request.
    ///
    /// # Returns
    /// A panel that renders the current help content through upstream Markdown.
    pub(crate) fn new(input_sender: Sender<ComposerInput>) -> Self {
        let mut content = String::from("# ScribeRig help\n\n");
        content.push_str("## Composer editing\n\n| Key | Action |\n| --- | --- |\n");
        let keybindings = rpi_tui::get_keybindings().clone();
        append_editor_bindings(&mut content, &keybindings);
        content.push_str("\n## ScribeRig controls\n\n| Key | Action |\n| --- | --- |\n");
        for (key, action) in super::super::controls::control_rows(&keybindings) {
            let _ = writeln!(content, "| {key} | {action} | ");
        }
        content.push('\n');
        content.push_str("## Semantic view navigation\n\n");
        content.push_str(
            "When browsing, use Up/Down to move, Enter to activate, and Esc to return to the composer. In conversation view, Left/Right changes branch expansion.\n\n## Commands\n\n",
        );
        for command in crate::slash_commands::COMMANDS {
            let name = crate::inspector::sanitize(command.name);
            let argument_hint = crate::inspector::sanitize(command.argument_hint);
            let summary = crate::inspector::sanitize(command.summary);
            let _ = writeln!(content, "- `{name}{argument_hint}`: {summary}");
        }
        let help_keys = keybindings
            .get_keys(super::super::controls::HELP)
            .iter()
            .map(rpi_tui::KeyCombo::display)
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(
            content,
            "\nPress Esc or {} to close this help panel.",
            crate::inspector::sanitize(&help_keys)
        );
        Self {
            markdown: Markdown::new(content, 1, 0),
            input_sender,
        }
    }
}

/// Appends active upstream editor bindings to the help document.
///
/// - `content`: Markdown document receiving the binding rows.
/// - `keybindings`: Upstream binding snapshot used by the editors.
fn append_editor_bindings(content: &mut String, keybindings: &Keybindings) {
    let mut bindings = keybindings
        .iter()
        .filter(|(id, _)| {
            id.starts_with("tui.editor.")
                || id.starts_with("tui.input.")
                || id.starts_with("tui.select.")
        })
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    bindings.sort_unstable();
    for binding in bindings {
        append_binding(content, keybindings, binding);
    }
}

/// Appends one upstream binding's configured key and description.
///
/// - `content`: Markdown document receiving the binding row.
/// - `keybindings`: Upstream binding snapshot used by the editors.
/// - `binding_id`: Upstream identifier to describe.
fn append_binding(content: &mut String, keybindings: &Keybindings, binding_id: KeybindingId) {
    let Some(definition) = keybindings.get_definition(binding_id) else {
        return;
    };
    let keys = keybindings
        .get_keys(binding_id)
        .iter()
        .map(rpi_tui::KeyCombo::display)
        .collect::<Vec<_>>()
        .join(", ");
    if keys.is_empty() {
        return;
    }
    let key = crate::inspector::sanitize(&keys);
    let action = definition
        .description
        .map_or_else(|| binding_id.to_owned(), crate::inspector::sanitize);
    let _ = writeln!(content, "| {key} | {action} |");
}

impl Component for HelpPanel {
    /// Renders generated binding documentation through upstream Markdown.
    ///
    /// # Parameters
    /// - `width`: Available render width in terminal columns.
    ///
    /// # Returns
    /// Rendered Markdown lines.
    fn render(&self, width: usize) -> Vec<String> {
        self.markdown.render(width)
    }

    /// Closes help on Escape or the configured help binding.
    ///
    /// # Parameters
    /// - `event`: Normalized upstream terminal event.
    ///
    /// # Returns
    /// `true` when a close key was consumed.
    fn handle_event(&self, event: &InputEvent) -> bool {
        let InputEvent::Key(key) = event else {
            return false;
        };
        if rpi_tui::get_keybindings().matches(key, rpi_tui::keybindings::keys::SELECT_CANCEL)
            || rpi_tui::get_keybindings().matches(key, super::super::controls::HELP)
        {
            let _ = self.input_sender.send(ComposerInput::CloseHelp);
            return true;
        }
        false
    }

    /// Invalidates upstream Markdown's cached frame.
    fn invalidate(&self) {
        self.markdown.invalidate();
    }

    /// Supports upstream layout-node inspection.
    ///
    /// # Returns
    /// A type-erased reference to this component.
    fn as_any(&self) -> &dyn Any {
        self
    }
}
