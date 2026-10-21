//! Adapts `ScribeRig` input and review state to upstream overlay components.

use std::any::Any;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use rpi_tui::tui::{OverlayOptions, SizeValue};
use rpi_tui::{
    Component, Dialog, Editor, EditorOptions, EditorStyle, Focusable, Markdown, TUI, TuiAltScreen,
    VStack,
};

use crate::action::Action;
use crate::effect::Effect;
use crate::overlay::{ConfirmationState, Overlay, input::InputPurpose};
use crate::state::TuiState;

use super::ComposerInput;
use super::event_loop::focus::restore_overlay_focus;

/// Applies an upstream input-dialog event through `ScribeRig` reducers.
///
/// # Parameters
/// - `input`: Modal event emitted by the upstream editor adapter.
/// - `state`: `ScribeRig` state receiving the input or semantic action.
///
/// # Returns
/// Modal effects when the event belongs to the input dialog, or `None` for
/// unrelated composer events.
pub(super) fn reduce_modal_input(
    input: ComposerInput,
    state: &mut TuiState,
) -> Option<Vec<Effect>> {
    match input {
        ComposerInput::ModalInputChanged(text) => {
            if let Some(Overlay::Input(input)) = state.overlay.as_mut() {
                input.text = text;
                input.error = None;
            }
            Some(Vec::new())
        }
        ComposerInput::ModalInputSubmitted(text) => {
            if let Some(Overlay::Input(input)) = state.overlay.as_mut() {
                input.text = text;
            }
            Some(crate::state::reduce(state, Action::InputSubmitted))
        }
        ComposerInput::ModalInputCancelled => {
            Some(crate::state::reduce(state, Action::OverlayClosed))
        }
        ComposerInput::ModalConfirmationAccepted(commit) => Some(crate::state::reduce(
            state,
            Action::CommitConfirmed(*commit),
        )),
        ComposerInput::ModalConfirmationCancelled(commit) => {
            let matches_confirmation = matches!(
                state.overlay.as_ref(),
                Some(Overlay::Confirm(confirmation)) if confirmation.commit == *commit
            );
            if matches_confirmation {
                Some(crate::state::reduce(state, Action::OverlayClosed))
            } else {
                Some(Vec::new())
            }
        }
        _ => None,
    }
}

/// Owns the active upstream component for a `ScribeRig` modal state.
pub(super) struct ModalOverlayController {
    /// Current upstream modal adapter, when a supported semantic modal is open.
    active: Option<ActiveOverlay>,
}

impl ModalOverlayController {
    /// Creates a controller with no active modal.
    ///
    /// # Returns
    /// An empty upstream modal controller.
    pub(super) fn new() -> Self {
        Self { active: None }
    }

    /// Keeps the upstream modal stack synchronized with `ScribeRig` state.
    ///
    /// # Parameters
    /// - `screen`: Upstream runtime owning focus and overlays.
    /// - `state`: Shared `ScribeRig` semantic modal state.
    /// - `editor`: Main composer whose focus is restored after the modal.
    /// - `input_sender`: Channel carrying modal intents to the semantic loop.
    /// - `focus_before_overlay`: Previously focused component for restoration.
    pub(super) fn synchronize(
        &mut self,
        screen: &TuiAltScreen,
        state: &Arc<Mutex<TuiState>>,
        editor: &Editor,
        input_sender: &Sender<ComposerInput>,
        focus_before_overlay: &mut Option<Arc<dyn Component>>,
    ) {
        let desired = state.lock().ok().and_then(|state| match &state.overlay {
            Some(Overlay::Input(input)) => Some(DesiredOverlay::Input {
                purpose: input.purpose,
                text: input.text.clone(),
            }),
            Some(Overlay::Confirm(confirmation)) => {
                Some(DesiredOverlay::Confirmation(confirmation.clone()))
            }
            _ => None,
        });

        if self
            .active
            .as_ref()
            .is_some_and(|active| !active.matches(desired.as_ref()))
        {
            self.close(screen, editor, focus_before_overlay);
        }
        if self.active.is_some() || desired.is_none() {
            return;
        }

        if focus_before_overlay.is_none() {
            *focus_before_overlay = screen.get_focus();
        }
        let component: Arc<dyn Component> = match desired.expect("checked present desired modal") {
            DesiredOverlay::Input { purpose, text } => {
                let component = Arc::new(input_overlay(
                    purpose,
                    &text,
                    Arc::clone(state),
                    input_sender.clone(),
                ));
                self.active = Some(ActiveOverlay::Input { purpose });
                component as Arc<dyn Component>
            }
            DesiredOverlay::Confirmation(confirmation) => {
                let component = Arc::new(confirmation_dialog(&confirmation, input_sender));
                self.active = Some(ActiveOverlay::Confirmation {
                    confirmation: Box::new(confirmation),
                });
                component as Arc<dyn Component>
            }
        };
        screen.set_focus(Some(Arc::clone(&component)));
        screen.show_overlay(
            component,
            Some(OverlayOptions {
                width: Some(SizeValue::Percent(0.72)),
                max_height: Some(SizeValue::Percent(0.72)),
                ..OverlayOptions::default()
            }),
        );
        editor.set_focused(false);
    }

    /// Closes this controller's modal and restores the previous focus.
    ///
    /// # Parameters
    /// - `screen`: Upstream runtime owning focus and overlays.
    /// - `editor`: Main composer whose focus is restored.
    /// - `focus_before_overlay`: Previously focused component.
    fn close(
        &mut self,
        screen: &TuiAltScreen,
        editor: &Editor,
        focus_before_overlay: &mut Option<Arc<dyn Component>>,
    ) {
        if self.active.take().is_some() {
            restore_overlay_focus(screen, editor, focus_before_overlay);
        }
    }
}

/// Identifies modal state used to preserve editor contents across redraws.
enum DesiredOverlay {
    /// Freeform input whose `ScribeRig` purpose controls submission semantics.
    Input { purpose: InputPurpose, text: String },
    /// Exact `ScribeRig` confirmation payload awaiting an explicit choice.
    Confirmation(ConfirmationState),
}

/// Holds the upstream component that currently owns modal focus.
enum ActiveOverlay {
    /// Upstream editor adapted to one semantic input purpose.
    Input {
        /// `ScribeRig` input purpose represented by the editor.
        purpose: InputPurpose,
    },
    /// Upstream dialog adapted to one exact reviewed commit.
    Confirmation {
        /// Confirmation data held by the `ScribeRig` review state.
        confirmation: Box<ConfirmationState>,
    },
}

impl ActiveOverlay {
    /// Checks whether the currently rendered adapter represents current state.
    ///
    /// # Parameters
    /// - `self`: Current upstream modal adapter.
    /// - `desired`: Current `ScribeRig` modal state.
    ///
    /// # Returns
    /// Whether focus and editing state can be retained.
    fn matches(&self, desired: Option<&DesiredOverlay>) -> bool {
        match (self, desired) {
            (
                Self::Input { purpose, .. },
                Some(DesiredOverlay::Input {
                    purpose: desired, ..
                }),
            ) => purpose == desired,
            (
                Self::Confirmation { confirmation, .. },
                Some(DesiredOverlay::Confirmation(desired)),
            ) => confirmation.as_ref() == desired,
            _ => false,
        }
    }
}

/// Builds a `ScribeRig` semantic heading above an upstream editor and routes
/// input through the upstream stack's focused-child support.
///
/// # Parameters
/// - `purpose`: Semantic purpose displayed above the editor.
/// - `text`: Initial value placed into the upstream editor.
/// - `state`: Shared state used to project validation feedback.
/// - `input_sender`: Channel for forwarding editor intents.
///
/// # Returns
/// An upstream layout containing a semantic heading and focused editor.
fn input_overlay(
    purpose: InputPurpose,
    text: &str,
    state: Arc<Mutex<TuiState>>,
    input_sender: Sender<ComposerInput>,
) -> VStack {
    let editor = Arc::new(Editor::new(
        EditorOptions {
            placeholder: Some("Enter to submit · Esc to cancel".to_owned()),
            ..EditorOptions::default()
        },
        EditorStyle::default(),
        Arc::new(rpi_tui::get_keybindings().clone()),
    ));
    editor.set_text(text);
    let changed_sender = input_sender.clone();
    editor.on_change(Arc::new(move |value| {
        let safe = crate::inspector::sanitize_external_text(value);
        let _ = changed_sender.send(ComposerInput::ModalInputChanged(safe));
    }));
    let submitted_sender = input_sender.clone();
    editor.on_submit(Arc::new(move |value| {
        let safe = crate::inspector::sanitize_external_text(value);
        let _ = submitted_sender.send(ComposerInput::ModalInputSubmitted(safe));
    }));
    let cancelled_sender = input_sender;
    editor.on_cancel(Arc::new(move || {
        let _ = cancelled_sender.send(ComposerInput::ModalInputCancelled);
    }));

    let heading: Arc<dyn Component> = Arc::new(InputModalHeading {
        purpose,
        markdown: Markdown::new("", 0, 0),
        state,
    });
    let editor_component: Arc<dyn Component> = editor;
    let layout = VStack::new();
    layout.add_child(heading);
    layout.add_child(Arc::clone(&editor_component));
    layout.set_focused_child(Some(editor_component));
    layout
}

/// ScribeRig-specific semantic context projected through upstream Markdown.
struct InputModalHeading {
    purpose: InputPurpose,
    markdown: Markdown,
    state: Arc<Mutex<TuiState>>,
}

impl InputModalHeading {
    /// Projects the active semantic heading and validation feedback.
    ///
    /// # Returns
    /// Sanitized Markdown content for the current overlay.
    fn content(&self) -> String {
        let title = match self.purpose {
            InputPurpose::Entity => "Inspect entity",
            InputPurpose::DiffSnapshot => "Choose diff snapshot",
            InputPurpose::FindQuery => "Find semantic content",
            InputPurpose::ProjectionEdit => "Edit source projection",
        };
        let mut content = format!("## {title}\n\nEnter a value and press Enter to continue.");
        if let Ok(state) = self.state.lock()
            && let Some(Overlay::Input(input)) = state.overlay.as_ref()
            && let Some(error) = input.error.as_deref()
        {
            content.push_str("\n\n**Validation:** ");
            content.push_str(&crate::inspector::sanitize(error));
        }
        content
    }
}

impl Component for InputModalHeading {
    /// Renders current semantic heading content through upstream Markdown.
    ///
    /// - `width`: Available render width in terminal columns.
    ///
    /// # Returns
    /// Rendered heading lines.
    fn render(&self, width: usize) -> Vec<String> {
        let content = self.content();
        if self.markdown.get_content() != content {
            self.markdown.set_content(content);
        }
        self.markdown.render(width)
    }

    /// Invalidates the cached upstream Markdown frame.
    fn invalidate(&self) {
        self.markdown.invalidate();
    }

    /// Supports upstream component inspection.
    ///
    /// # Returns
    /// A type-erased reference to this heading.
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Creates the upstream dialog wired to the exact reviewed commit identity.
///
/// # Parameters
/// - `confirmation`: Exact reviewed `ScribeRig` operation and snapshot data.
/// - `input_sender`: Channel for reporting the selected confirmation action.
///
/// # Returns
/// An upstream dialog that reports acceptance or cancellation.
fn confirmation_dialog(
    confirmation: &ConfirmationState,
    input_sender: &Sender<ComposerInput>,
) -> Dialog {
    let action = crate::inspector::sanitize(&confirmation.action);
    let target = crate::inspector::sanitize(&confirmation.target);
    let base = crate::inspector::sanitize(&confirmation.base);
    let summary = crate::inspector::sanitize(&confirmation.summary);
    let candidate = crate::inspector::sanitize(&confirmation.commit.candidate_snapshot.to_string());
    let message = format!(
        "Action: {action}\nTarget: {target}\nReviewed base: {base}\nCandidate result: {candidate}\n{summary}"
    );
    let dialog = Dialog::new(
        "Confirm reviewed change",
        message,
        vec!["Commit".to_owned(), "Cancel".to_owned()],
    );
    let commit = confirmation.commit.clone();
    let sender = input_sender.clone();
    dialog.on_submit(Arc::new(move |selected| {
        let input = if selected == 0 {
            ComposerInput::ModalConfirmationAccepted(Box::new(commit.clone()))
        } else {
            ComposerInput::ModalConfirmationCancelled(Box::new(commit.clone()))
        };
        let _ = sender.send(input);
    }));
    let commit = confirmation.commit.clone();
    let sender = input_sender.clone();
    dialog.on_cancel(Arc::new(move || {
        let _ = sender.send(ComposerInput::ModalConfirmationCancelled(Box::new(
            commit.clone(),
        )));
    }));
    dialog
}
