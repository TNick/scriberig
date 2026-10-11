//! Stores presentation-only state for the initial terminal shell.

use crate::{
    action::{Action, Direction},
    effect::Effect,
    event::TuiEvent,
    explorer::ExplorerModel,
    inspector::{InspectorModel, LoadState},
    navigation::NavigationState,
    overlay::{input::InputState, palette::PaletteState, Overlay},
    selection::EntitySelection,
};
use scriberig_app::RuntimeCapability;

/// The visible shell pane receiving keyboard focus.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Focus {
    /// The semantic explorer pane.
    Explorer,
    /// The primary content pane.
    Primary,
    /// The inspector pane.
    Inspector,
}

impl Focus {
    /// Moves focus through the shell's stable pane order.
    /// - `reverse`: Argument named `reverse`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    fn next(self, reverse: bool) -> Self {
        let panes = [Self::Explorer, Self::Primary, Self::Inspector];
        let index = panes.iter().position(|pane| *pane == self).unwrap_or(1);
        panes[(index + if reverse { 2 } else { 1 }) % panes.len()]
    }
}

/// Runtime and presentation state for the initial TUI shell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiState {
    /// Whether the event loop should terminate.
    pub should_quit: bool,
    /// The most recently observed terminal dimensions.
    pub terminal_size: (u16, u16),
    /// The pane that currently owns local keyboard focus.
    pub focus: Focus,
    /// A short status message rendered in the shell.
    pub shell_message: String,
    /// Semantic navigation history, independent from pane focus.
    pub navigation: NavigationState,
    /// The active modal overlay, if one is open.
    pub overlay: Option<Overlay>,
    /// Presentation cursor index.
    pub cursor: usize,
    /// Stable semantic selection independent of the visible cursor.
    pub selection: Option<EntitySelection>,
    /// Semantic explorer model.
    pub explorer: ExplorerModel,
    /// Current inspector request state.
    pub inspector: LoadState<InspectorModel>,
    /// Runtime capabilities supplied by the application adapter.
    pub capabilities: Vec<RuntimeCapability>,
    /// Next request identifier.
    pub next_request_id: u64,
    /// Selected primary view.
    pub primary_view: crate::selection::PrimaryView,
    /// History request/result state.
    pub history: LoadState<crate::view::history::HistoryModel>,
    /// Diff request/result state.
    pub diff: LoadState<crate::view::diff::DiffModel>,
    /// Projection request/result state.
    pub projection: LoadState<crate::view::projection::ProjectionModel>,
    /// Change review request/result state.
    pub review: LoadState<crate::view::changes::ChangeReviewModel>,
    /// Last operation failure retained for review and refresh feedback.
    pub operation_error: Option<(String, String)>,
    /// Last reviewed proposal retained after a failed confirmation.
    pub review_proposal: Option<crate::view::changes::ChangeReviewModel>,
    /// Operation observations retained for the operations view.
    pub operations: Vec<crate::view::operations::OperationModel>,
}

impl Default for TuiState {
    /// Creates an empty, deterministic shell state.
    ///
    /// # Returns
    /// The value produced by the declaration.
    fn default() -> Self {
        Self {
            should_quit: false,
            terminal_size: (80, 24),
            focus: Focus::Primary,
            shell_message: "No semantic service is attached in this runtime.".to_owned(),
            navigation: NavigationState::default(),
            overlay: None,
            cursor: 0,
            selection: None,
            explorer: ExplorerModel::default(),
            inspector: LoadState::NotRequested,
            capabilities: Vec::new(),
            next_request_id: 1,
            primary_view: crate::selection::PrimaryView::Overview,
            history: LoadState::NotRequested,
            diff: LoadState::NotRequested,
            projection: LoadState::NotRequested,
            review: LoadState::NotRequested,
            operation_error: None,
            review_proposal: None,
            operations: Vec::new(),
        }
    }
}

impl TuiState {
    /// Applies one local terminal event to presentation state.
    /// - `event`: Argument named `event`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn apply_event(&mut self, event: &TuiEvent) -> Vec<Effect> {
        match event {
            TuiEvent::Key(key) => {
                if let Some(action) = crate::keymap::action_for(self, key) {
                    return reduce(self, action);
                }
            }
            TuiEvent::Resize { width, height } => {
                return reduce(
                    self,
                    Action::TerminalResized {
                        width: *width,
                        height: *height,
                    },
                );
            }
            TuiEvent::Shutdown => return reduce(self, Action::QuitRequested),
            TuiEvent::Paste(text) => {
                return reduce(self, Action::Paste(text.clone()));
            }
            TuiEvent::Application(event) => return reduce_application_event(self, event),
            TuiEvent::Tick => {}
        }
        Vec::new()
    }

    /// Reports whether a text input currently owns keyboard focus.
    #[must_use]
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn is_input_active(&self) -> bool {
        matches!(self.overlay, Some(Overlay::Input(_)))
    }

    /// Returns the current input text, when an input overlay is active.
    #[must_use]
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn input_text(&self) -> Option<&str> {
        match self.overlay.as_ref() {
            Some(Overlay::Input(input)) => Some(&input.text),
            _ => None,
        }
    }
}

/// Applies one action deterministically and returns work for the runtime.
#[allow(clippy::too_many_lines)]
/// - `state`: Argument named `state`.
/// - `action`: Argument named `action`.
///
/// # Returns
/// The value produced by the declaration.
pub fn reduce(state: &mut TuiState, action: Action) -> Vec<Effect> {
    match action {
        Action::Noop => vec![Effect::None],
        Action::QuitRequested => {
            state.should_quit = true;
            vec![Effect::RequestQuit]
        }
        Action::OverlayClosed | Action::CancelReview => {
            state.overlay = None;
            vec![Effect::None]
        }
        Action::FocusNext => {
            if state.overlay.is_none() {
                state.focus = state.focus.next(false);
            }
            vec![Effect::None]
        }
        Action::FocusPrevious => {
            if state.overlay.is_none() {
                state.focus = state.focus.next(true);
            }
            vec![Effect::None]
        }
        Action::CursorMoved(direction) => {
            if let Some(Overlay::Palette(palette)) = state.overlay.as_mut() {
                let last = palette.filtered().len().saturating_sub(1);
                palette.selected = match direction {
                    Direction::Previous => palette.selected.saturating_sub(1),
                    Direction::Next => palette.selected.saturating_add(1).min(last),
                };
            } else {
                state.cursor = match direction {
                    Direction::Previous => state.cursor.saturating_sub(1),
                    Direction::Next => state.cursor.saturating_add(1),
                };
            }
            vec![Effect::None]
        }
        Action::Activated => activate_current(state),
        Action::ActivateLocation(location) => {
            state.navigation.activate(location);
            vec![Effect::None]
        }
        Action::Back => {
            state.navigation.go_back();
            vec![Effect::None]
        }
        Action::Forward => {
            state.navigation.go_forward();
            vec![Effect::None]
        }
        Action::RefreshRequested => {
            let Some(selection) = state.selection else {
                return vec![Effect::None];
            };
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.inspector = LoadState::Loading { request_id };
            vec![Effect::InspectEntity {
                request_id,
                entity: selection.entity,
                snapshot: selection.snapshot,
            }]
        }
        Action::HelpOpened => {
            state.overlay = Some(Overlay::Help);
            vec![Effect::None]
        }
        Action::PaletteOpened => {
            state.overlay = Some(Overlay::Palette(PaletteState::with_capabilities(
                &state.capabilities,
            )));
            vec![Effect::None]
        }
        Action::FindOpened => {
            state.overlay = Some(Overlay::Input(InputState::default()));
            vec![Effect::None]
        }
        Action::InputChanged(text) => {
            if let Some(Overlay::Input(input)) = state.overlay.as_mut() {
                input.error = None;
                if text.is_empty() {
                    input.backspace();
                } else {
                    input.append(&text);
                }
            } else if let Some(Overlay::Palette(palette)) = state.overlay.as_mut() {
                if text.is_empty() {
                    palette.query.pop();
                    palette.selected = palette
                        .selected
                        .min(palette.filtered().len().saturating_sub(1));
                } else {
                    palette.set_query(format!("{}{}", palette.query, text));
                }
            }
            vec![Effect::None]
        }
        Action::Paste(text) => {
            if let Some(Overlay::Input(input)) = state.overlay.as_mut() {
                input.append(&text);
            } else if let Some(Overlay::Palette(palette)) = state.overlay.as_mut() {
                palette.set_query(format!("{}{}", palette.query, text));
            }
            vec![Effect::None]
        }
        Action::InputSubmitted => {
            let text = state.input_text().unwrap_or_default().trim().to_owned();
            if state.is_input_active() && !text.is_empty() {
                match text.parse::<scriberig_core::EntityId>() {
                    Ok(entity) => {
                        let request_id = state.next_request_id;
                        state.next_request_id += 1;
                        state.overlay = None;
                        return reduce(
                            state,
                            Action::InspectRequested {
                                request_id,
                                entity,
                                snapshot: None,
                            },
                        );
                    }
                    Err(error) => {
                        if let Some(Overlay::Input(input)) = state.overlay.as_mut() {
                            input.error = Some(format!("invalid entity identifier: {error}"));
                        }
                    }
                }
            }
            vec![Effect::None]
        }
        Action::TerminalResized { width, height } => {
            state.terminal_size = (width, height);
            vec![Effect::None]
        }
        Action::InspectRequested {
            request_id,
            entity,
            snapshot,
        } => {
            state.selection = Some(EntitySelection {
                entity,
                revision: None,
                snapshot,
            });
            state.explorer = ExplorerModel::from_selection(EntitySelection {
                entity,
                revision: None,
                snapshot,
            });
            state.inspector = LoadState::Loading { request_id };
            vec![Effect::InspectEntity {
                request_id,
                entity,
                snapshot,
            }]
        }
        Action::InspectSucceeded { request_id, model } => {
            if matches!(state.inspector, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.selection = Some(EntitySelection {
                    entity: model.entity,
                    revision: model.revision,
                    snapshot: model.snapshot,
                });
                state.inspector = LoadState::Ready(model.clone());
                state
                    .navigation
                    .activate(crate::selection::NavigationLocation {
                        selection: Some(crate::selection::SemanticReference(
                            model.entity.to_string(),
                        )),
                        entity: state.selection,
                        ..Default::default()
                    });
            }
            vec![Effect::None]
        }
        Action::InspectFailed {
            request_id,
            category,
            message,
        } => {
            if matches!(state.inspector, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.inspector = LoadState::Failed {
                    category,
                    message,
                    stale: false,
                };
            }
            vec![Effect::None]
        }
        Action::CapabilitiesLoaded(capabilities) => {
            state.capabilities = capabilities;
            vec![Effect::None]
        }
        Action::ViewSelected(view) => {
            state.primary_view = view;
            vec![Effect::None]
        }
        Action::HistorySucceeded { request_id, model } => {
            if matches!(state.history, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.history = LoadState::Ready(model);
            }
            vec![Effect::None]
        }
        Action::DiffSucceeded { request_id, model } => {
            if matches!(state.diff, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.diff = LoadState::Ready(model);
            }
            vec![Effect::None]
        }
        Action::ProjectionSucceeded { request_id, model } => {
            if matches!(state.projection, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.projection = LoadState::Ready(model);
            }
            vec![Effect::None]
        }
        Action::ReviewSucceeded { request_id, model } => {
            if matches!(state.review, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.review = LoadState::Ready(model);
            }
            vec![Effect::None]
        }
        Action::OperationFailed {
            request_id,
            category,
            message,
        } => {
            state.operation_error = Some((category.clone(), message.clone()));
            if matches!(state.history, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.history = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            if matches!(state.diff, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.diff = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            if matches!(state.projection, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.projection = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            if matches!(state.review, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.review = LoadState::Failed {
                    category,
                    message,
                    stale: state.review_proposal.is_some(),
                };
            }
            vec![Effect::None]
        }
        Action::ConfirmReview => {
            if let Some(Overlay::Confirm(confirm)) = state.overlay.take() {
                return reduce(state, Action::CommitConfirmed(confirm.commit));
            }
            if let LoadState::Ready(model) = &state.review {
                let commit = crate::view::changes::ConfirmedCommit {
                    base_snapshot: model.base_snapshot,
                    change: model.change.clone(),
                    branch: model.branch.clone(),
                };
                state.overlay = Some(Overlay::Confirm(crate::overlay::ConfirmationState {
                    action: "commit change".to_owned(),
                    target: model.change.clone(),
                    base: model.base_snapshot.to_string(),
                    summary: format!(
                        "{} affected semantic objects",
                        model.affected_entities.len()
                    ),
                    commit,
                }));
            }
            vec![Effect::None]
        }
        Action::CommitConfirmed(commit) => {
            state.overlay = None;
            if let LoadState::Ready(model) = &state.review {
                state.review_proposal = Some(model.clone());
            }
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.review = LoadState::Loading { request_id };
            vec![Effect::Commit {
                request_id,
                request: scriberig_app::CommitRequest {
                    base_snapshot: commit.base_snapshot,
                    branch: commit.branch,
                    change: commit.change,
                },
            }]
        }
        Action::HistoryRequested { snapshot } => {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.primary_view = crate::selection::PrimaryView::History;
            state.history = LoadState::Loading { request_id };
            vec![Effect::History {
                request_id,
                snapshot,
            }]
        }
        Action::DiffRequested { request } => {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.primary_view = crate::selection::PrimaryView::Diff;
            state.diff = LoadState::Loading { request_id };
            vec![Effect::Diff {
                request_id,
                request,
            }]
        }
        Action::ProjectionRequested { request } => {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.primary_view = crate::selection::PrimaryView::Projection;
            state.projection = LoadState::Loading { request_id };
            vec![Effect::Render {
                request_id,
                request,
            }]
        }
        Action::ReviewRequested { request } => {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.primary_view = crate::selection::PrimaryView::Changes;
            state.review = LoadState::Loading { request_id };
            vec![Effect::Apply {
                request_id,
                request,
            }]
        }
        Action::OperationObserved(model) => {
            if let Some(existing) = state.operations.iter_mut().find(|item| item.id == model.id) {
                *existing = model;
            } else {
                state.operations.push(model);
            }
            vec![Effect::None]
        }
        Action::OperationStatusRequested(operation) => {
            vec![Effect::OperationStatus { operation }]
        }
        Action::OperationCancelRequested(operation) => {
            vec![Effect::CancelOperation { operation }]
        }
    }
}

/// Converts shared events into bounded local operation observations.
/// - `state`: Argument named `state`.
/// - `event`: Argument named `event`.
///
/// # Returns
/// The value produced by the declaration.
fn reduce_application_event(
    state: &mut TuiState,
    event: &scriberig_app::ApplicationEvent,
) -> Vec<Effect> {
    let operation = match event {
        scriberig_app::ApplicationEvent::OperationStarted { operation }
        | scriberig_app::ApplicationEvent::OperationProgressed { operation, .. }
        | scriberig_app::ApplicationEvent::OperationCompleted { operation } => operation.clone(),
        _ => return vec![Effect::None],
    };
    reduce(state, Action::OperationStatusRequested(operation))
}

/// Activates the selected local palette item without invoking semantic services.
/// - `state`: Argument named `state`.
///
/// # Returns
/// The value produced by the declaration.
fn activate_current(state: &mut TuiState) -> Vec<Effect> {
    let Some(Overlay::Palette(palette)) = state.overlay.as_ref() else {
        return vec![Effect::None];
    };
    let filtered = palette.filtered();
    let Some(item) = filtered.get(palette.selected) else {
        return vec![Effect::None];
    };
    let action = match item.id.as_str() {
        "help" => Action::HelpOpened,
        "refresh" => Action::RefreshRequested,
        "back" => Action::Back,
        "quit" => Action::QuitRequested,
        "inspect_entity" => {
            state.overlay = Some(Overlay::Input(InputState::default()));
            return vec![Effect::None];
        }
        "overview" => Action::ViewSelected(crate::selection::PrimaryView::Overview),
        "history" => {
            let Some(snapshot) = state.selection.and_then(|selection| selection.snapshot) else {
                return vec![Effect::None];
            };
            return reduce(state, Action::HistoryRequested { snapshot });
        }
        "diff" | "projection" | "changes" => {
            return vec![Effect::None];
        }
        _ => return vec![Effect::None],
    };
    state.overlay = None;
    reduce(state, action)
}

#[cfg(test)]
mod tests {
    use super::{reduce, Focus, TuiState};
    use crate::action::{Action, Direction};
    use crate::effect::Effect;
    use crate::event::{Key, KeyCode, Modifiers, TuiEvent};
    use crate::overlay::{palette::PaletteState, Overlay};

    /// Verifies resize events update only terminal presentation state.
    #[test]
    fn resize_updates_state() {
        let mut state = TuiState::default();
        state.apply_event(&TuiEvent::Resize {
            width: 72,
            height: 24,
        });
        assert_eq!(state.terminal_size, (72, 24));
        assert!(!state.should_quit);
    }

    /// Verifies quit input stops the runtime.
    #[test]
    fn quit_key_sets_quit_flag() {
        let mut state = TuiState::default();
        state.apply_event(&TuiEvent::Key(Key {
            code: KeyCode::Char('q'),
            modifiers: Modifiers::default(),
        }));
        assert!(state.should_quit);
    }

    /// Verifies modal input takes precedence over browse commands.
    #[test]
    fn input_consumes_printable_keys() {
        let mut state = TuiState::default();
        reduce(&mut state, Action::FindOpened);
        state.apply_event(&TuiEvent::Key(Key {
            code: KeyCode::Char('q'),
            modifiers: Modifiers::default(),
        }));
        assert_eq!(state.input_text(), Some("q"));
        assert!(!state.should_quit);
    }

    /// Verifies focus cycles without changing semantic history.
    #[test]
    fn focus_cycles_independently_of_navigation() {
        let mut state = TuiState::default();
        let history = state.navigation.clone();
        reduce(&mut state, Action::FocusNext);
        assert_eq!(state.focus, Focus::Inspector);
        reduce(&mut state, Action::FocusNext);
        assert_eq!(state.focus, Focus::Explorer);
        reduce(&mut state, Action::FocusNext);
        assert_eq!(state.focus, Focus::Primary);
        reduce(&mut state, Action::FocusPrevious);
        assert_eq!(state.focus, Focus::Explorer);
        assert_eq!(state.navigation, history);
    }

    /// Verifies overlays own input and close without changing navigation.
    #[test]
    fn overlay_closes_without_navigation_change() {
        let mut state = TuiState::default();
        let history = state.navigation.clone();
        reduce(&mut state, Action::HelpOpened);
        reduce(&mut state, Action::OverlayClosed);
        assert!(state.overlay.is_none());
        assert_eq!(state.navigation, history);
    }

    /// Verifies paste is inserted as one text payload.
    #[test]
    fn paste_is_input_data() {
        let mut state = TuiState::default();
        reduce(&mut state, Action::FindOpened);
        reduce(&mut state, Action::Paste("q\n?".into()));
        assert_eq!(state.input_text(), Some("q\n?"));
        assert!(!state.should_quit);
    }

    /// Verifies pasted text is retained by the command palette.
    #[test]
    fn palette_paste_updates_filter_text() {
        let mut state = TuiState::default();
        reduce(&mut state, Action::PaletteOpened);
        reduce(&mut state, Action::Paste("REF".into()));
        assert_eq!(
            state.overlay,
            Some(Overlay::Palette(PaletteState {
                query: "REF".to_owned(),
                ..PaletteState::default()
            }))
        );
    }

    /// Verifies palette cursor movement and activation stay local to the overlay.
    #[test]
    fn palette_selection_moves_and_activates() {
        let mut state = TuiState::default();
        reduce(&mut state, Action::PaletteOpened);
        reduce(&mut state, Action::CursorMoved(Direction::Next));
        assert_eq!(
            state.overlay.as_ref().and_then(|overlay| match overlay {
                Overlay::Palette(palette) => Some(palette.selected),
                _ => None,
            }),
            Some(1)
        );
        let effects = reduce(&mut state, Action::Activated);
        assert_eq!(effects, vec![Effect::None]);
        assert!(state.overlay.is_none());
    }

    /// Verifies refresh re-reads the selected entity through the service port.
    #[test]
    fn refresh_reissues_selected_inspection() {
        let mut state = TuiState::default();
        let entity = scriberig_core::EntityId::from_bytes([7; 16]);
        state.selection = Some(crate::selection::EntitySelection {
            entity,
            revision: None,
            snapshot: None,
        });
        let effects = reduce(&mut state, Action::RefreshRequested);
        assert!(
            matches!(effects.as_slice(), [Effect::InspectEntity { entity: selected, .. }] if *selected == entity)
        );
    }

    /// Verifies application lifecycle events request authoritative operation state.
    #[test]
    fn application_operation_event_requests_authoritative_refresh() {
        let mut state = TuiState::default();
        let operation = scriberig_core::OperationId::new("running").expect("operation id");
        let effects = state.apply_event(&crate::event::TuiEvent::Application(
            scriberig_app::ApplicationEvent::OperationStarted {
                operation: operation.clone(),
            },
        ));
        assert_eq!(effects, vec![Effect::OperationStatus { operation }]);
    }

    /// Verifies closing the operations presentation cannot cancel application work.
    #[test]
    fn closing_overlay_does_not_cancel_operation() {
        let mut state = TuiState::default();
        let operation = scriberig_core::OperationId::new("active").expect("operation id");
        let record =
            scriberig_app::OperationRecord::queued(operation, scriberig_app::OperationKind::Query);
        reduce(
            &mut state,
            Action::OperationObserved(crate::view::operations::OperationModel::from_record(
                &record,
            )),
        );
        assert!(state.operations[0].cancellable());
        assert_eq!(
            reduce(&mut state, Action::OverlayClosed),
            vec![Effect::None]
        );
        assert!(state.operations[0].cancellable());
    }

    /// Verifies resizing preserves semantic navigation history.
    #[test]
    fn resize_preserves_navigation() {
        let mut state = TuiState::default();
        reduce(
            &mut state,
            Action::ActivateLocation(crate::selection::NavigationLocation::default()),
        );
        let history = state.navigation.clone();
        reduce(
            &mut state,
            Action::TerminalResized {
                width: 20,
                height: 8,
            },
        );
        assert_eq!(state.navigation, history);
    }

    /// Verifies invalid go-to input remains local and emits no inspect effect.
    #[test]
    fn invalid_entity_input_stays_in_overlay() {
        let mut state = TuiState::default();
        reduce(&mut state, Action::FindOpened);
        reduce(&mut state, Action::InputChanged("not-an-entity".to_owned()));
        let effects = reduce(&mut state, Action::InputSubmitted);
        assert_eq!(effects, vec![Effect::None]);
        assert!(matches!(state.overlay, Some(Overlay::Input(input)) if input.error.is_some()));
        assert!(matches!(
            state.inspector,
            crate::inspector::LoadState::NotRequested
        ));
    }

    /// Verifies a late result cannot overwrite a newer inspector request.
    #[test]
    fn stale_inspect_result_is_ignored() {
        let mut state = TuiState::default();
        let entity = scriberig_core::EntityId::from_bytes([4; 16]);
        reduce(
            &mut state,
            Action::InspectRequested {
                request_id: 1,
                entity,
                snapshot: None,
            },
        );
        reduce(
            &mut state,
            Action::InspectRequested {
                request_id: 2,
                entity,
                snapshot: None,
            },
        );
        let model = crate::inspector::InspectorModel::from_result(scriberig_app::InspectResult {
            entity,
            revision: None,
            snapshot: None,
            object_type: None,
            label: Some("old".to_owned()),
            description: None,
            capabilities: Vec::new(),
            trust: None,
        });
        reduce(
            &mut state,
            Action::InspectSucceeded {
                request_id: 1,
                model,
            },
        );
        assert!(matches!(
            state.inspector,
            crate::inspector::LoadState::Loading { request_id: 2 }
        ));
    }

    /// Verifies switching primary views retains semantic selection.
    #[test]
    fn view_switch_preserves_selection() {
        let mut state = TuiState::default();
        let entity = scriberig_core::EntityId::from_bytes([8; 16]);
        state.selection = Some(crate::selection::EntitySelection {
            entity,
            revision: None,
            snapshot: None,
        });
        reduce(
            &mut state,
            Action::ViewSelected(crate::selection::PrimaryView::History),
        );
        assert_eq!(state.selection.map(|value| value.entity), Some(entity));
    }

    /// Verifies review confirmation is local until the confirmed command is emitted.
    #[test]
    fn confirmation_does_not_mutate_without_explicit_confirm() {
        let mut state = TuiState::default();
        let base = scriberig_core::SnapshotId::new([9; 32]);
        state.review =
            crate::inspector::LoadState::Ready(crate::view::changes::ChangeReviewModel {
                base_snapshot: base,
                change: "change-1".to_owned(),
                branch: None,
                affected_entities: Vec::new(),
                conflicts: Vec::new(),
                validation: crate::view::changes::ReviewValidation::Passed,
                requires_confirmation: true,
            });
        assert_eq!(
            reduce(&mut state, Action::ConfirmReview),
            vec![Effect::None]
        );
        assert!(matches!(state.overlay, Some(Overlay::Confirm(_))));
        assert!(matches!(
            state.review,
            crate::inspector::LoadState::Ready(_)
        ));
    }

    /// Verifies the confirmed commit carries the reviewed base snapshot.
    #[test]
    fn confirmed_commit_preserves_reviewed_base() {
        let mut state = TuiState::default();
        let base = scriberig_core::SnapshotId::new([6; 32]);
        let effects = reduce(
            &mut state,
            Action::CommitConfirmed(crate::view::changes::ConfirmedCommit {
                base_snapshot: base,
                change: "change-2".to_owned(),
                branch: None,
            }),
        );
        assert!(
            matches!(effects.as_slice(), [Effect::Commit { request_id: 1, request }]
            if request.base_snapshot == base && request.change == "change-2")
        );
    }

    /// Verifies a stale operation failure remains visible and is not retried.
    #[test]
    fn stale_operation_failure_is_visible_without_retry() {
        let mut state = TuiState::default();
        reduce(
            &mut state,
            Action::ReviewRequested {
                request: scriberig_app::ApplyRequest {
                    base_snapshot: scriberig_core::SnapshotId::new([1; 32]),
                    branch: None,
                    change: "change".to_owned(),
                },
            },
        );
        let effects = reduce(
            &mut state,
            Action::OperationFailed {
                request_id: 1,
                category: "stale_base_snapshot".to_owned(),
                message: "base changed".to_owned(),
            },
        );
        assert_eq!(effects, vec![Effect::None]);
        assert!(
            matches!(state.review, crate::inspector::LoadState::Failed { ref category, .. } if category == "stale_base_snapshot")
        );
    }
}
