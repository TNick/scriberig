//! Builds upstream selectors from credential-safe `ScribeRig` model rows.

use std::sync::{Arc, Mutex, mpsc::Sender};

use rpi_tui::tui::{OverlayOptions, SizeValue};
use rpi_tui::{Component, SearchableSelectList, SelectItem, SelectList, TUI};

use crate::state::TuiState;

use super::{ComposerInput, ControlKind, SelectorChoice};
#[cfg(test)]
#[path = "selector_factory/large_catalog_tests.rs"]
mod large_catalog_tests;
#[path = "selector_factory/model_items.rs"]
mod model_items;
#[path = "selector_factory/readiness.rs"]
mod readiness;

use model_items::model_selector_items;
use readiness::{provider_status, safe_selector_text};

/// Opens the built-in upstream theme presets in the upstream selector widget.
///
/// # Parameters
/// - `screen`: Upstream TUI that owns the overlay lifecycle.
/// - `input_sender`: Channel receiving selected or cancelled rows.
///
/// # Returns
/// `true` when the preset selector was opened.
pub(super) fn open_theme_selector(
    screen: &Arc<rpi_tui::TuiAltScreen>,
    input_sender: &Sender<ComposerInput>,
) -> bool {
    let items = rpi_tui::ThemePreset::ALL
        .into_iter()
        .map(|preset| {
            SelectItem::new(preset.id(), preset.label()).with_description(preset.description())
        })
        .collect();
    let list = Arc::new(SelectList::new(items, 8));
    let sender = input_sender.clone();
    list.on_select(Arc::new(move |item| {
        let _ = sender.send(ComposerInput::Selected(SelectorChoice::Theme(
            item.value.clone(),
        )));
    }));
    let sender = input_sender.clone();
    list.on_cancel(Arc::new(move || {
        let _ = sender.send(ComposerInput::CancelSelection);
    }));
    let component: Arc<dyn Component> = list;
    screen.set_focus(Some(Arc::clone(&component)));
    screen.show_overlay(
        component,
        Some(OverlayOptions {
            width: Some(SizeValue::Percent(0.62)),
            max_height: Some(SizeValue::Percent(0.62)),
            ..OverlayOptions::default()
        }),
    );
    true
}

/// Opens the command palette through upstream searchable selector components.
///
/// # Parameters
/// - `screen`: Upstream TUI that owns the overlay lifecycle.
/// - `state`: `ScribeRig` presentation state containing the capability-filtered palette.
/// - `input_sender`: Channel receiving selected or cancelled actions.
/// - `plugin_commands`: Admitted extension commands included in the palette.
///
/// # Returns
/// `true` when an overlay was opened.
pub(super) fn open_palette_selector(
    screen: &Arc<rpi_tui::TuiAltScreen>,
    state: &Arc<Mutex<TuiState>>,
    input_sender: &Sender<ComposerInput>,
    plugin_commands: &[rpi_extensions::isolated::WorkerCommandSpec],
) -> bool {
    let items = {
        let Ok(state) = state.lock() else {
            return false;
        };
        let Some(crate::overlay::Overlay::Palette(palette)) = state.overlay.as_ref() else {
            return false;
        };
        palette
            .local_actions
            .iter()
            .chain(&palette.semantic_actions)
            .map(|item| SelectItem::new(&item.id, &safe_selector_text(&item.label)))
            .collect::<Vec<_>>()
    };
    let mut items = items;
    items.extend(plugin_palette_items(plugin_commands));
    if items.is_empty() {
        return false;
    }
    let list = Arc::new(SelectList::new(items, 16));
    let sender = input_sender.clone();
    list.on_select(Arc::new(move |item| {
        let _ = sender.send(ComposerInput::PaletteSelected(item.value.clone()));
    }));
    let sender = input_sender.clone();
    list.on_cancel(Arc::new(move || {
        let _ = sender.send(ComposerInput::CancelSelection);
    }));
    let searchable = Arc::new(SearchableSelectList::new(
        Some("Command palette"),
        Some("Search ScribeRig actions"),
        list,
    ));
    let component: Arc<dyn Component> = searchable;
    screen.set_focus(Some(Arc::clone(&component)));
    screen.show_overlay(
        component,
        Some(OverlayOptions {
            width: Some(SizeValue::Percent(0.82)),
            max_height: Some(SizeValue::Percent(0.82)),
            ..OverlayOptions::default()
        }),
    );
    true
}

/// Projects upstream-registered plugin commands into safe palette rows.
///
/// - `commands`: Host-authorized registrations from the isolated extension worker.
///
/// # Returns
/// Stable, sanitized selector rows excluding names reserved by product commands.
fn plugin_palette_items(
    commands: &[rpi_extensions::isolated::WorkerCommandSpec],
) -> Vec<SelectItem> {
    commands
        .iter()
        .filter(|command| {
            valid_plugin_command_name(&command.name)
                && crate::slash_commands::find(&format!("/{}", command.name)).is_none()
        })
        .map(|command| {
            SelectItem::new(
                &format!("plugin-command:{}", command.name),
                &safe_selector_text(&format!("/{} — {}", command.name, command.description)),
            )
        })
        .collect()
}

/// Checks whether an upstream command can be represented as one slash token.
///
/// - `name`: Host-advertised command identity without a leading slash.
///
/// # Returns
/// `true` when the name is safe for palette and composer routing.
pub(super) fn valid_plugin_command_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

/// Opens an upstream selector populated from credential-safe model metadata.
///
/// # Parameters
/// - `screen`: Upstream TUI that owns the overlay lifecycle.
/// - `state`: `ScribeRig` state used to resolve current session assignment.
/// - `kind`: Provider, model, or thinking-level selector to show.
/// - `library`: Safe model-library rows returned by the shared application.
/// - `provider_filter`: Optional provider identity limiting model rows.
/// - `input_sender`: Channel receiving selected or cancelled rows.
///
/// # Returns
/// `true` when an overlay was opened.
pub(super) fn open_control_selector(
    screen: &Arc<rpi_tui::TuiAltScreen>,
    state: &Arc<Mutex<TuiState>>,
    kind: ControlKind,
    library: &scriberig_protocol::ModelLibraryResponse,
    provider_filter: Option<&str>,
    input_sender: &Sender<ComposerInput>,
) -> bool {
    let selection = selector_items(kind, state, library, provider_filter);
    let SelectorDefinition {
        items,
        title,
        description,
        searchable,
    } = selection;

    if items.is_empty() {
        if let Ok(mut state) = state.lock() {
            state.shell_message = match kind {
                ControlKind::Provider => "No providers are available.".to_owned(),
                ControlKind::Model => "No models are available for that provider.".to_owned(),
                ControlKind::PlanAssignment => {
                    "No models are available for plan assignment.".to_owned()
                }
                ControlKind::Thinking => {
                    "The selected model has no available thinking levels.".to_owned()
                }
            };
        }
        screen.request_render(false);
        return false;
    }

    let list = Arc::new(SelectList::new(items, 12));
    let sender = input_sender.clone();
    list.on_select(Arc::new(move |item| {
        let choice = match kind {
            ControlKind::Provider => SelectorChoice::Provider(item.value.clone()),
            ControlKind::Model => SelectorChoice::Model(item.value.clone()),
            ControlKind::PlanAssignment => SelectorChoice::PlanModel(item.value.clone()),
            ControlKind::Thinking => SelectorChoice::Thinking(item.value.clone()),
        };
        let _ = sender.send(ComposerInput::Selected(choice));
    }));
    let sender = input_sender.clone();
    list.on_cancel(Arc::new(move || {
        let _ = sender.send(ComposerInput::CancelSelection);
    }));

    let component: Arc<dyn Component> = if searchable {
        Arc::new(SearchableSelectList::new(
            Some(title),
            Some(description),
            list,
        ))
    } else {
        list
    };
    screen.set_focus(Some(Arc::clone(&component)));
    screen.show_overlay(
        component,
        Some(OverlayOptions {
            width: Some(SizeValue::Percent(0.82)),
            max_height: Some(SizeValue::Percent(0.82)),
            ..OverlayOptions::default()
        }),
    );
    true
}

/// Builds upstream items and presentation metadata for a model control.
///
/// # Parameters
/// - `kind`: Provider, model, or thinking-level selector to build.
/// - `state`: `ScribeRig` state used to resolve the selected model.
/// - `library`: Credential-safe model-library rows.
/// - `provider_filter`: Optional provider identity limiting model rows.
///
/// # Returns
/// Native selector items with their title, description, and search behavior.
fn selector_items(
    kind: ControlKind,
    state: &Arc<Mutex<TuiState>>,
    library: &scriberig_protocol::ModelLibraryResponse,
    provider_filter: Option<&str>,
) -> SelectorDefinition {
    let mut items = Vec::new();
    let (title, description, searchable) = match kind {
        ControlKind::Provider => {
            let mut providers = std::collections::BTreeMap::<String, usize>::new();
            for model in &library.models {
                *providers.entry(model.provider_id.clone()).or_default() += 1;
            }
            items.extend(providers.into_iter().map(|(provider, count)| {
                let safe_provider = safe_selector_text(&provider);
                let status = provider_status(library, &provider);
                SelectItem::new(&provider, &safe_provider)
                    .with_description(&format!("{count} models · {status}"))
                    .with_search_text(&safe_provider)
            }));
            (
                "Select provider",
                "Choose a provider, then choose one of its models.",
                false,
            )
        }
        ControlKind::Model | ControlKind::PlanAssignment => {
            items.extend(model_selector_items(library, provider_filter));
            let title = match (kind, provider_filter.is_some()) {
                (ControlKind::PlanAssignment, _) => "Assign plan node model",
                (_, true) => "Select provider model",
                _ => "Select model",
            };
            let description = if matches!(kind, ControlKind::PlanAssignment) {
                "Choose the upstream model assigned to this plan node."
            } else {
                "Choose the model for future session invocations."
            };
            (title, description, true)
        }
        ControlKind::Thinking => {
            let selection = state.lock().ok().and_then(|state| {
                if let crate::inspector::LoadState::Ready(session) = &state.session {
                    Some((
                        session.execution.provider_id.clone()?,
                        session.execution.model_id.clone()?,
                    ))
                } else {
                    None
                }
            });
            if let Some((provider, model_id)) = selection {
                if let Some(model) = library.models.iter().find(|model| {
                    model.provider_id == provider && model.provider_model_id == model_id
                }) {
                    items.extend(model.thinking_levels.iter().map(|level| {
                        let label = safe_selector_text(level);
                        SelectItem::new(level, &label)
                    }));
                }
            }
            (
                "Select thinking level",
                "Choose a level advertised by the selected model.",
                false,
            )
        }
    };
    SelectorDefinition {
        items,
        title,
        description,
        searchable,
    }
}

/// Holds selector rows and labels prepared from `ScribeRig` model metadata.
///
/// # Public members
/// None.
///
/// # Private members
/// - `items`: Native upstream selector rows.
/// - `title`: Selector title.
/// - `description`: Selector instruction text.
/// - `searchable`: Whether to wrap rows in upstream search behavior.
struct SelectorDefinition {
    /// Native upstream selector rows.
    items: Vec<SelectItem>,
    /// Selector title.
    title: &'static str,
    /// Selector instruction text.
    description: &'static str,
    /// Whether to wrap rows in upstream search behavior.
    searchable: bool,
}

#[cfg(test)]
mod plugin_command_tests {
    use super::{plugin_palette_items, valid_plugin_command_name};

    /// Restricts plugin command routing to one unambiguous slash token.
    #[test]
    fn plugin_command_name_validation_rejects_path_and_ambiguous_tokens() {
        assert!(valid_plugin_command_name("status_check-2"));
        assert!(!valid_plugin_command_name("../status"));
        assert!(!valid_plugin_command_name("2status"));
        assert!(!valid_plugin_command_name("two words"));
    }

    /// Includes upstream commands in the palette while avoiding built-in aliases.
    #[test]
    fn plugin_commands_share_the_palette_and_respect_builtin_names() {
        let rows = plugin_palette_items(&[
            rpi_extensions::isolated::WorkerCommandSpec {
                name: "fixture-status".to_owned(),
                description: "Show status".to_owned(),
            },
            rpi_extensions::isolated::WorkerCommandSpec {
                name: "model".to_owned(),
                description: "Reserved product command".to_owned(),
            },
        ]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, "plugin-command:fixture-status");
        assert!(rows[0].label.contains("Show status"));
    }
}
