//! Adapts `ScribeRig` composer/status controls to upstream TUI components.

use std::any::Any;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use rpi_tui::{Component, Editor, Focusable, FooterComponent, InputEvent, Keybindings};

use crate::state::TuiState;

use super::ComposerInput;

#[path = "components/help_panel.rs"]
mod help_panel;
pub(super) use help_panel::HelpPanel;
#[path = "components/session_list.rs"]
mod session_list;
pub(super) use session_list::SessionListPanel;

/// Adapts focused terminal events to an upstream editor.
///
/// # Public members
/// None.
///
/// # Private members
/// - `editor`: Upstream multiline editor for cursor and text behavior.
/// - `input_sender`: Channel for submitted drafts and shutdown.
/// - `keybindings`: Upstream keybinding manager including `ScribeRig` actions.
pub(super) struct ComposerAdapter {
    /// Upstream multiline editor used for cursor and text behavior.
    pub(super) editor: Arc<Editor>,
    /// Channel used to request shutdown after Ctrl-C.
    input_sender: std::sync::mpsc::Sender<ComposerInput>,
    /// Shared upstream bindings used for controls and help text.
    keybindings: Arc<Keybindings>,
}

impl ComposerAdapter {
    /// Creates an event-forwarding adapter around the upstream editor.
    ///
    /// # Parameters
    /// - `editor`: Upstream multiline editor.
    /// - `input_sender`: Channel used to deliver drafts to the reducer.
    /// - `keybindings`: Upstream bindings for `ScribeRig` controls.
    ///
    /// # Returns
    /// An adapter that forwards input and editor changes.
    pub(super) fn new(
        editor: Arc<Editor>,
        input_sender: std::sync::mpsc::Sender<ComposerInput>,
        keybindings: Arc<Keybindings>,
    ) -> Self {
        let submit_sender = input_sender.clone();
        editor.on_submit(Arc::new(move |prompt| {
            let _ = submit_sender.send(ComposerInput::Submit(prompt.to_owned()));
        }));
        let change_sender = input_sender.clone();
        editor.on_change(Arc::new(move |draft| {
            let _ = change_sender.send(ComposerInput::DraftChanged(draft.to_owned()));
        }));
        Self {
            editor,
            input_sender,
            keybindings,
        }
    }
}

impl Component for ComposerAdapter {
    /// Updates the upstream editor's focused state.
    ///
    /// - `focused`: Whether the editor currently owns input focus.
    fn set_focus_state(&self, focused: bool) {
        self.editor.set_focused(focused);
    }

    /// Renders the upstream editor without flattening or reconstructing styles.
    ///
    /// # Parameters
    /// - `width`: Available render width in terminal columns.
    ///
    /// # Returns
    /// Rendered editor lines.
    fn render(&self, width: usize) -> Vec<String> {
        self.editor.render(width)
    }

    /// Forwards normalized upstream key and paste events to the editor.
    ///
    /// # Parameters
    /// - `event`: Normalized event supplied by the upstream terminal reader.
    ///
    /// # Returns
    /// `true` when the composer consumed the event.
    fn handle_event(&self, event: &InputEvent) -> bool {
        if let InputEvent::Key(key) = event {
            if self.keybindings.matches(key, super::controls::BROWSE) {
                let _ = self
                    .input_sender
                    .send(ComposerInput::TogglePresentationFocus(true));
                return true;
            }
            if self.keybindings.matches(key, super::controls::PROVIDER) {
                let _ = self
                    .input_sender
                    .send(ComposerInput::OpenControl(super::ControlKind::Provider));
                return true;
            }
            if self.keybindings.matches(key, super::controls::MODEL) {
                let _ = self
                    .input_sender
                    .send(ComposerInput::OpenControl(super::ControlKind::Model));
                return true;
            }
            if self.keybindings.matches(key, super::controls::THINKING) {
                let _ = self
                    .input_sender
                    .send(ComposerInput::OpenControl(super::ControlKind::Thinking));
                return true;
            }
            if self
                .keybindings
                .matches(key, super::controls::MODEL_PROPERTIES)
            {
                let _ = self.input_sender.send(ComposerInput::OpenModelProperties);
                return true;
            }
            if self.keybindings.matches(key, super::controls::THEME) {
                let _ = self.input_sender.send(ComposerInput::OpenThemeSelector);
                return true;
            }
            if self.keybindings.matches(key, super::controls::PALETTE) {
                let _ = self.input_sender.send(ComposerInput::OpenPalette);
                return true;
            }
            if self.keybindings.matches(key, super::controls::HELP) {
                let _ = self.input_sender.send(ComposerInput::OpenHelp);
                return true;
            }
        }
        match event {
            InputEvent::Key(key) => self.editor.handle_key(*key),
            InputEvent::Paste(text) => {
                self.editor.handle_paste(text);
                true
            }
            _ => false,
        }
    }

    /// Invalidates the upstream editor component.
    fn invalidate(&self) {
        self.editor.invalidate();
    }

    /// Supports upstream layout-node inspection.
    ///
    /// # Returns
    /// A type-erased reference to this component.
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Focusable for ComposerAdapter {
    /// Updates the upstream editor's focus marker.
    ///
    /// # Parameters
    /// - `focused`: Whether the editor should receive focus.
    fn set_focused(&self, focused: bool) {
        self.editor.set_focused(focused);
    }

    /// Reports focus from the upstream editor.
    ///
    /// # Returns
    /// Whether the editor is currently focused.
    fn is_focused(&self) -> bool {
        self.editor.is_focused()
    }
}

/// Supplies session status to the upstream footer component.
///
/// # Public members
/// None.
///
/// # Private members
/// - `state`: Shared `ScribeRig` session assignment and status.
/// - `footer`: Upstream footer and status renderer.
/// - `hints`: Full active key-label text.
pub(super) struct SelectionStatusPanel {
    /// Shared `ScribeRig` session assignment and status.
    state: Arc<Mutex<TuiState>>,
    /// Upstream footer and status renderer.
    footer: FooterComponent,
    /// Full active key-label text retained so status rows can truncate it.
    hints: String,
}

impl SelectionStatusPanel {
    /// Creates an upstream footer backed by current `ScribeRig` state.
    ///
    /// # Parameters
    /// - `state`: Shared presentation state.
    /// - `hints`: Active key labels from the upstream binding manager.
    ///
    /// # Returns
    /// A panel backed by the supplied presentation state.
    pub(super) fn new(state: Arc<Mutex<TuiState>>, hints: impl AsRef<str>) -> Self {
        let footer = FooterComponent::new();
        footer.set_model("");
        footer.set_hints(hints.as_ref());
        Self {
            state,
            footer,
            hints: hints.as_ref().to_owned(),
        }
    }

    /// Builds safe assignment and readiness text for the upstream footer.
    ///
    /// # Parameters
    /// - `state`: Current `ScribeRig` presentation state.
    ///
    /// # Returns
    /// Safe status text for the upstream footer renderer.
    fn content(state: &TuiState) -> String {
        let mut content = String::new();
        if let crate::inspector::LoadState::Ready(session) = &state.session {
            let provider = session
                .execution
                .provider_id
                .as_deref()
                .map_or_else(|| "unassigned".to_owned(), crate::inspector::sanitize);
            let model = session
                .execution
                .model_id
                .as_deref()
                .map_or_else(|| "unassigned".to_owned(), crate::inspector::sanitize);
            let _ = write!(content, "Provider {provider} · model {model}");
            if let (Some(provider_id), Some(model_id)) = (
                session.execution.provider_id.as_deref(),
                session.execution.model_id.as_deref(),
            ) && let crate::inspector::LoadState::Ready(library) = &state.model_library
            {
                let status = library
                    .provider_readiness
                    .iter()
                    .find(|status| status.provider_id == provider_id && status.model_id == model_id)
                    .map_or("credential status unavailable", |status| {
                        match status.credential {
                            scriberig_protocol::ProviderCredentialReadiness::NotRequired => {
                                "authentication not required"
                            }
                            scriberig_protocol::ProviderCredentialReadiness::Available => {
                                "credential available"
                            }
                            scriberig_protocol::ProviderCredentialReadiness::Missing => {
                                "credential missing"
                            }
                            scriberig_protocol::ProviderCredentialReadiness::Denied => {
                                "authorization denied"
                            }
                            scriberig_protocol::ProviderCredentialReadiness::Unsupported => {
                                "credential source unsupported"
                            }
                            scriberig_protocol::ProviderCredentialReadiness::CatalogHeader => {
                                "catalog authentication configured"
                            }
                        }
                    });
                let _ = write!(content, " · auth {status}");
            }
            if let Some(thinking) = session.execution.thinking_level.as_deref() {
                let _ = write!(
                    content,
                    " · thinking {}",
                    crate::inspector::sanitize(thinking)
                );
            }
        } else {
            content.push_str("No active session assignment");
        }
        if state.conversation_browsing {
            if !content.is_empty() {
                content.push_str(" · ");
            }
            content.push_str("Enter pin reply target");
        }
        if !state.shell_message.is_empty() {
            if !content.is_empty() {
                content.push_str(" · ");
            }
            content.push_str(&crate::inspector::sanitize(&state.shell_message));
        }
        content
    }
}

impl Component for SelectionStatusPanel {
    /// Renders `ScribeRig` status through the upstream footer component.
    ///
    /// # Parameters
    /// - `width`: Available render width in terminal columns.
    ///
    /// # Returns
    /// Rendered upstream footer lines.
    fn render(&self, width: usize) -> Vec<String> {
        self.footer.invalidate();
        if let Ok(state) = self.state.lock() {
            let content = Self::content(&state);
            let hint_width = width.saturating_sub(content.chars().count().saturating_add(1));
            self.footer
                .set_hints(&self.hints.chars().take(hint_width).collect::<String>());
            self.footer.set_status(&content);
            if let crate::inspector::LoadState::Ready(session) = &state.session {
                let model = session
                    .execution
                    .model_id
                    .as_deref()
                    .map(crate::inspector::sanitize)
                    .unwrap_or_default();
                self.footer.set_model(&model);
                self.footer
                    .set_thinking_level(session.execution.thinking_level.as_deref());
            } else {
                self.footer.set_model("");
                self.footer.clear_thinking_level();
            }
        }
        self.footer.render(width)
    }

    /// Invalidates the upstream footer's cached frame.
    fn invalidate(&self) {
        self.footer.invalidate();
    }

    /// Supports upstream layout-node inspection.
    ///
    /// # Returns
    /// A type-erased reference to this component.
    fn as_any(&self) -> &dyn Any {
        self
    }
}
