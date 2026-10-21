//! Registers `ScribeRig` actions with the upstream keybinding manager.

use std::io;

use crossterm::event::{KeyCode, KeyModifiers};
use rpi_tui::{KeyCombo, KeybindingDefinition, KeybindingId, Keybindings};

const PREFIX: &str = "scriberig.";

pub(super) const HELP: KeybindingId = "scriberig.help";
pub(super) const PALETTE: KeybindingId = "scriberig.palette";
pub(super) const PROVIDER: KeybindingId = "scriberig.provider.select";
pub(super) const MODEL: KeybindingId = "scriberig.model.select";
pub(super) const THINKING: KeybindingId = "scriberig.thinking.select";
pub(super) const MODEL_PROPERTIES: KeybindingId = "scriberig.model.properties";
pub(super) const THEME: KeybindingId = "scriberig.theme.select";
pub(super) const BROWSE: KeybindingId = "scriberig.view.browse";
pub(super) const QUIT: KeybindingId = "scriberig.quit";

/// Adds `ScribeRig` product actions to an upstream keybinding manager.
///
/// # Parameters
/// - `keybindings`: Upstream manager receiving `ScribeRig` action definitions.
///
/// # Returns
/// `Ok(())` after every product binding has been registered.
///
/// # Errors
/// Returns an error if an identifier is already used by another definition.
pub(super) fn register_scriberig_bindings(keybindings: &mut Keybindings) -> io::Result<()> {
    register(
        keybindings,
        HELP,
        vec![
            KeyCombo::from_code(KeyCode::F(1)),
            KeyCombo::new(KeyCode::Char('?'), KeyModifiers::SHIFT),
        ],
        "Help",
    )?;
    register(
        keybindings,
        PALETTE,
        vec![KeyCombo::new(KeyCode::Char('p'), KeyModifiers::CONTROL)],
        "Palette",
    )?;
    register(
        keybindings,
        PROVIDER,
        vec![KeyCombo::from_code(KeyCode::F(2))],
        "Provider",
    )?;
    register(
        keybindings,
        MODEL,
        vec![KeyCombo::from_code(KeyCode::F(3))],
        "Model",
    )?;
    register(
        keybindings,
        THINKING,
        vec![KeyCombo::from_code(KeyCode::F(4))],
        "Thinking",
    )?;
    register(
        keybindings,
        MODEL_PROPERTIES,
        vec![KeyCombo::from_code(KeyCode::F(5))],
        "Properties",
    )?;
    register(
        keybindings,
        THEME,
        vec![KeyCombo::from_code(KeyCode::F(6))],
        "Theme",
    )?;
    register(
        keybindings,
        BROWSE,
        vec![KeyCombo::from_code(KeyCode::F(7))],
        "Browse",
    )?;
    register(
        keybindings,
        QUIT,
        vec![KeyCombo::new(KeyCode::Char('c'), KeyModifiers::CONTROL)],
        "Quit",
    )
}

/// Registers one `ScribeRig` control with upstream conflict tracking.
///
/// - `keybindings`: Upstream manager receiving the definition.
/// - `id`: Stable `ScribeRig` action identifier.
/// - `default_keys`: Default key combinations for the action.
/// - `description`: User-facing action description.
///
/// # Returns
/// `Ok(())` when the binding is registered.
///
/// # Errors
/// Returns an I/O error when the ID conflicts with an existing definition.
fn register(
    keybindings: &mut Keybindings,
    id: KeybindingId,
    default_keys: Vec<KeyCombo>,
    description: &'static str,
) -> io::Result<()> {
    keybindings
        .register(
            id,
            KeybindingDefinition {
                default_keys,
                description: Some(description),
            },
        )
        .map_err(io::Error::other)
}

/// Returns configured `ScribeRig` action keys with their descriptions.
///
/// - `keybindings`: Upstream manager containing registered `ScribeRig` actions.
///
/// Configured key labels and action descriptions in stable identifier order.
/// # Returns
/// One display row per configured product binding.
pub(super) fn control_rows(keybindings: &Keybindings) -> Vec<(String, String)> {
    let mut bindings = keybindings
        .iter()
        .filter(|(id, _)| id.starts_with(PREFIX))
        .collect::<Vec<_>>();
    bindings.sort_by_key(|(id, _)| *id);
    bindings
        .into_iter()
        .filter_map(|(id, definition)| {
            let keys = keybindings
                .get_keys(id)
                .iter()
                .map(KeyCombo::display)
                .collect::<Vec<_>>()
                .join(", ");
            if keys.is_empty() {
                return None;
            }
            let description = definition.description.unwrap_or(id);
            Some((
                crate::inspector::sanitize(&keys),
                crate::inspector::sanitize(description),
            ))
        })
        .collect()
}

/// Formats configured `ScribeRig` actions for the compact upstream footer.
///
/// - `keybindings`: Upstream manager containing registered `ScribeRig` actions.
///
/// A compact list of configured key and action labels.
/// # Returns
/// Key and action labels separated for compact display.
pub(super) fn control_hints(keybindings: &Keybindings) -> String {
    control_rows(keybindings)
        .into_iter()
        .map(|(key, action)| format!("{key} {action}"))
        .collect::<Vec<_>>()
        .join(" · ")
}
