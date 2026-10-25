//! Runs the conversation home through the upstream TUI lifecycle.

use std::io;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use rpi_tui::{
    Component, Editor, EditorOptions, EditorStyle, Focusable, Keybindings, ProcessTerminal, TUI,
    TuiAltScreen,
};

use crate::debug_log::TuiDebugLog;
use crate::service::TuiService;
use crate::state::TuiState;

#[path = "upstream/components.rs"]
mod components;
#[path = "upstream/controls.rs"]
mod controls;
#[path = "upstream/event_loop.rs"]
mod event_loop;
#[cfg(test)]
#[path = "upstream/lifecycle_tests.rs"]
mod lifecycle_tests;
#[path = "upstream/modal_overlay.rs"]
mod modal_overlay;
#[path = "upstream/property_editor.rs"]
mod property_editor;
#[path = "upstream/selector_factory.rs"]
mod selector_factory;
#[path = "upstream/semantic_views.rs"]
mod semantic_views;
#[path = "upstream/shell_panes.rs"]
mod shell_panes;
#[cfg(test)]
#[path = "upstream/transcript_follow_tests.rs"]
mod transcript_follow_tests;

use components::{ComposerAdapter, SelectionStatusPanel};
use semantic_views::SemanticViewPanel;
use shell_panes::ShellPanes;

/// Retains the current `ScribeRig` semantic-list row identity.
///
/// - `state`: Presentation state receiving the selected row identity.
pub(super) fn retain_semantic_row_cursor(state: &mut TuiState) {
    semantic_views::retain_cursor_identity(state);
}

/// Reconciles the semantic-list cursor after its projection changes.
///
/// - `state`: Presentation state receiving the refreshed selection and cursor.
pub(super) fn reconcile_semantic_row_cursor(state: &mut TuiState) {
    semantic_views::reconcile_cursor(state);
}

/// Retains the stable plan/node selection after upstream tree navigation.
///
/// - `state`: Presentation state receiving the selected plan identity.
pub(super) fn retain_plan_cursor(state: &mut TuiState) {
    semantic_views::retain_plan_cursor(state);
}

/// Reconciles the plan selection after an authoritative plan refresh.
///
/// - `state`: Presentation state receiving the refreshed plan position.
pub(super) fn reconcile_plan_cursor(state: &mut TuiState) {
    semantic_views::reconcile_plan_cursor(state);
}

/// One input produced by the focused upstream composer.
pub(super) enum ComposerInput {
    /// A complete draft submitted with Enter.
    Submit(String),
    /// The current draft changed while editing.
    DraftChanged(String),
    /// A request to stop the active TUI.
    Quit,
    /// Toggle keyboard routing between the upstream editor and semantic view.
    TogglePresentationFocus(bool),
    /// Apply a `ScribeRig` cursor or activation action from the active view.
    PresentationAction(Box<crate::action::Action>),
    /// Open one upstream-backed model control.
    OpenControl(ControlKind),
    /// Open upstream theme preset selection.
    OpenThemeSelector,
    /// Open schema-backed settings for the currently assigned model.
    OpenModelProperties,
    /// Start editing a free-form upstream model property.
    BeginPropertyEdit {
        provider_id: String,
        model_id: String,
        key: String,
        current_value: String,
    },
    /// Apply a finite choice selected in the upstream property editor.
    ModelPropertyChanged {
        provider_id: String,
        model_id: String,
        key: String,
        value: serde_json::Value,
    },
    /// Apply a free-form value submitted through the upstream editor.
    ModelPropertySubmitted {
        provider_id: String,
        model_id: String,
        key: String,
        value: serde_json::Value,
    },
    /// Close the upstream property settings surface.
    CancelPropertySettings,
    /// Cancel free-form property editing.
    CancelPropertyEdit,
    /// Open the upstream command palette.
    OpenPalette,
    /// Open `ScribeRig` keymap help in an upstream overlay.
    OpenHelp,
    /// Close the active upstream help overlay.
    CloseHelp,
    /// Apply a selected command palette row.
    PaletteSelected(String),
    /// Apply a selected upstream model control row.
    Selected(SelectorChoice),
    /// Cancel the active selector.
    CancelSelection,
    /// Update text in an upstream-backed `ScribeRig` input dialog.
    ModalInputChanged(String),
    /// Submit text from an upstream-backed `ScribeRig` input dialog.
    ModalInputSubmitted(String),
    /// Cancel the active upstream-backed `ScribeRig` input dialog.
    ModalInputCancelled,
    /// Accept the exact reviewed commit shown by the upstream dialog.
    ModalConfirmationAccepted(Box<crate::view::changes::ConfirmedCommit>),
    /// Cancel the exact pending confirmation shown by the upstream dialog.
    ModalConfirmationCancelled(Box<crate::view::changes::ConfirmedCommit>),
}

/// Creates the application-wide Ctrl-C handler installed on the upstream TUI.
///
/// - `input_sender`: Channel receiving the application quit intent.
/// - `keybindings`: Upstream bindings including the `ScribeRig` quit action.
/// - `state`: Shared presentation state for operation cancellation routing.
///
/// # Returns
/// A thread-safe upstream input callback.
fn global_quit_handler(
    input_sender: mpsc::Sender<ComposerInput>,
    keybindings: Arc<Keybindings>,
    state: Arc<Mutex<TuiState>>,
) -> Arc<dyn Fn(&rpi_tui::InputEvent) -> bool + Send + Sync> {
    Arc::new(move |event| {
        let rpi_tui::InputEvent::Key(key) = event else {
            return false;
        };
        if keybindings.matches(key, controls::QUIT) {
            let _ = input_sender.send(ComposerInput::Quit);
            true
        } else if keybindings.matches(key, controls::CANCEL_OPERATION)
            && let Ok(state) = state.lock()
            && state.primary_view == crate::selection::PrimaryView::Operations
            && let Some(operation) = state
                .operations
                .get(state.cursor)
                .filter(|operation| operation.cancellable())
                .or_else(|| {
                    state
                        .operations
                        .iter()
                        .rev()
                        .find(|operation| operation.kind == "Agent" && operation.cancellable())
                })
                .or_else(|| {
                    state
                        .operations
                        .iter()
                        .rev()
                        .find(|operation| operation.cancellable())
                })
        {
            input_sender
                .send(ComposerInput::PresentationAction(Box::new(
                    crate::action::Action::OperationCancelRequested(operation.id.clone()),
                )))
                .is_ok()
        } else {
            false
        }
    })
}

/// Identifies one model-control selector to open.
#[derive(Clone, Copy)]
pub(super) enum ControlKind {
    /// Choose a provider before choosing one of its models.
    Provider,
    /// Choose any available model.
    Model,
    /// Choose a model for the selected durable plan node.
    PlanAssignment,
    /// Choose a thinking level for the selected model.
    Thinking,
}

/// Carries a safe selected identifier from an upstream list callback.
pub(super) enum SelectorChoice {
    /// Stable provider identifier.
    Provider(String),
    /// Stable `ScribeRig` model-library identifier.
    Model(String),
    /// Stable model-library identifier assigned to a plan node.
    PlanModel(String),
    /// Advertised thinking level identifier.
    Thinking(String),
    /// Upstream theme preset identifier.
    Theme(String),
}

/// Runs the active conversation shell with upstream terminal ownership.
///
/// The upstream reader dispatches normalized input to the focused editor
/// adapter. This thread applies `ScribeRig` reducers and effects, then requests
/// upstream redraws from the shared semantic state.
///
/// # Parameters
/// - `service`: Shared `ScribeRig` application operation adapter.
/// - `context`: Authorization and snapshot context for application work.
/// - `config`: Initial project, agent scope, and semantic selection.
/// - `state`: Initial `ScribeRig` presentation state.
/// - `debug_log`: Optional redacted runtime log.
///
/// # Returns
/// Success after the user quits or an I/O failure occurs.
///
/// # Errors
/// Returns an error when runtime diagnostics cannot be written.
pub(super) fn run<S: TuiService + Sync>(
    service: &S,
    context: &scriberig_app::ApplicationContext,
    config: &crate::TuiConfig,
    state: TuiState,
    debug_log: Option<&mut TuiDebugLog>,
) -> io::Result<()> {
    let initial_view = state.primary_view;
    let shared_state = Arc::new(Mutex::new(state));
    let (input_sender, input_receiver) = mpsc::channel();
    let mut keybindings = rpi_tui::get_keybindings().clone();
    controls::register_scriberig_bindings(&mut keybindings)?;
    rpi_tui::set_keybindings(keybindings.clone());
    let keybindings = Arc::new(keybindings);
    let editor = Arc::new(Editor::new(
        EditorOptions {
            placeholder: Some(format!(
                "Enter send · {}",
                controls::control_hints(&keybindings)
            )),
            ..EditorOptions::default()
        },
        EditorStyle::default(),
        Arc::clone(&keybindings),
    ));
    let composer = Arc::new(ComposerAdapter::new(
        Arc::clone(&editor),
        input_sender.clone(),
        Arc::clone(&keybindings),
    ));
    let conversation = Arc::new(SemanticViewPanel::new(
        Arc::clone(&shared_state),
        input_sender.clone(),
        Arc::clone(&keybindings),
    ));
    let selection_status = Arc::new(SelectionStatusPanel::new(
        Arc::clone(&shared_state),
        controls::control_hints(&keybindings),
    ));
    let status_pane: Arc<dyn Component> = selection_status;
    let composer_pane: Arc<dyn Component> = Arc::clone(&composer) as Arc<dyn Component>;
    let mut panes = ShellPanes::new(&conversation, status_pane, composer_pane, initial_view);

    let screen = Arc::new(TuiAltScreen::new(
        Box::new(ProcessTerminal::new()),
        true,
        None,
    ));
    screen.set_global_input_handler(Some(global_quit_handler(
        input_sender.clone(),
        Arc::clone(&keybindings),
        Arc::clone(&shared_state),
    )));
    screen.set_layout_root(Some(panes.root()));
    composer.set_focused(true);
    let focused_composer: Arc<dyn Component> = composer.clone();
    screen.set_focus(Some(focused_composer));
    event_loop::run(event_loop::EventLoopContext {
        service,
        context,
        config,
        state: &shared_state,
        screen: &screen,
        panes: &mut panes,
        editor: &editor,
        composer: &composer,
        semantic_view: &conversation,
        input_sender: &input_sender,
        input_receiver: &input_receiver,
        debug_log,
    })
}
