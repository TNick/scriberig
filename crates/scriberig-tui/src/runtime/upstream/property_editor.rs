//! Adapts upstream schema-backed property controls for the local model settings UI.

use std::sync::Arc;
use std::sync::mpsc::Sender;

use rpi_tui::tui::{OverlayOptions, SizeValue};
use rpi_tui::{
    Component, Editor, EditorOptions, EditorStyle, Markdown, PropertyEditor, PropertyEditorChoice,
    PropertyEditorItem, TUI, TuiAltScreen, VStack,
};

use super::ComposerInput;

/// Opens the upstream property editor for schema-declared, non-sensitive rows.
///
/// # Parameters
/// - `screen`: Upstream terminal owning focus and overlay lifecycle.
/// - `provider_id`: Exact selected upstream provider identity.
/// - `model_id`: Exact selected provider-native model identity.
/// - `descriptors`: Upstream schema-backed editable properties.
/// - `input_sender`: Channel for model property changes.
///
/// # Returns
/// `true` when an editable property overlay is shown.
pub(super) fn open_property_editor(
    screen: &Arc<TuiAltScreen>,
    provider_id: &str,
    model_id: &str,
    descriptors: Vec<rpi_model_runtime::ModelPropertyDescriptor>,
    input_sender: &Sender<ComposerInput>,
) -> bool {
    let items = descriptors
        .into_iter()
        .filter_map(property_item)
        .collect::<Vec<_>>();
    if items.is_empty() {
        return false;
    }

    let editor = Arc::new(PropertyEditor::new(items));
    let sender = input_sender.clone();
    let provider = provider_id.to_owned();
    let model = model_id.to_owned();
    editor.on_change(Arc::new(move |key, value| {
        let _ = sender.send(ComposerInput::ModelPropertyChanged {
            provider_id: provider.clone(),
            model_id: model.clone(),
            key: key.to_owned(),
            value: parse_value(value),
        });
    }));
    let sender = input_sender.clone();
    let provider = provider_id.to_owned();
    let model = model_id.to_owned();
    editor.on_edit(Arc::new(move |item| {
        let _ = sender.send(ComposerInput::BeginPropertyEdit {
            provider_id: provider.clone(),
            model_id: model.clone(),
            key: item.key.clone(),
            current_value: item.value.clone(),
        });
    }));
    let sender = input_sender.clone();
    editor.on_cancel(Arc::new(move || {
        let _ = sender.send(ComposerInput::CancelPropertySettings);
    }));

    let heading = Arc::new(Markdown::new(
        format!(
            "## Model properties: {}/{}\n\nChanges use upstream schema validation.",
            safe_text(provider_id),
            safe_text(model_id)
        ),
        0,
        0,
    ));
    let layout = VStack::new();
    layout.add_child(heading);
    let editor_component: Arc<dyn Component> = editor;
    layout.add_child(Arc::clone(&editor_component));
    layout.set_focused_child(Some(editor_component));
    let component: Arc<dyn Component> = Arc::new(layout);
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

/// Opens upstream text editing for a non-enumerated property value.
///
/// # Parameters
/// - `screen`: Upstream terminal owning focus and overlay lifecycle.
/// - `provider_id`: Exact selected upstream provider identity.
/// - `model_id`: Exact selected provider-native model identity.
/// - `key`: Upstream schema property being edited.
/// - `current_value`: Existing safe property value.
/// - `input_sender`: Channel for editor completion and cancellation.
pub(super) fn open_property_value_editor(
    screen: &Arc<TuiAltScreen>,
    provider_id: &str,
    model_id: &str,
    key: &str,
    current_value: &str,
    input_sender: &Sender<ComposerInput>,
) {
    let editor = Arc::new(Editor::new(
        EditorOptions {
            placeholder: Some("Enter JSON or text | Enter save | Esc cancel".to_owned()),
            ..EditorOptions::default()
        },
        EditorStyle::default(),
        Arc::new(rpi_tui::get_keybindings().clone()),
    ));
    editor.set_text(current_value);
    let sender = input_sender.clone();
    let provider = provider_id.to_owned();
    let model = model_id.to_owned();
    let property = key.to_owned();
    editor.on_submit(Arc::new(move |text| {
        let _ = sender.send(ComposerInput::ModelPropertySubmitted {
            provider_id: provider.clone(),
            model_id: model.clone(),
            key: property.clone(),
            value: parse_value(text),
        });
    }));
    let cancelled_sender = input_sender.clone();
    editor.on_cancel(Arc::new(move || {
        let _ = cancelled_sender.send(ComposerInput::CancelPropertyEdit);
    }));

    let heading = Arc::new(Markdown::new(
        format!(
            "## Edit {}\n\nInput is validated by the upstream model schema.",
            safe_text(key)
        ),
        0,
        0,
    ));
    let layout = VStack::new();
    layout.add_child(heading);
    let editor_component: Arc<dyn Component> = editor;
    layout.add_child(Arc::clone(&editor_component));
    layout.set_focused_child(Some(editor_component));
    let component: Arc<dyn Component> = Arc::new(layout);
    screen.set_focus(Some(Arc::clone(&component)));
    screen.show_overlay(
        component,
        Some(OverlayOptions {
            width: Some(SizeValue::Percent(0.72)),
            max_height: Some(SizeValue::Percent(0.72)),
            ..OverlayOptions::default()
        }),
    );
}

/// Converts one public upstream descriptor into an editor row.
///
/// - `descriptor`: Upstream descriptor to validate and project.
///
/// # Returns
/// A generic editor row when the descriptor is safe to expose.
fn property_item(
    descriptor: rpi_model_runtime::ModelPropertyDescriptor,
) -> Option<PropertyEditorItem> {
    if !descriptor.is_safe_for_generic_editor() {
        return None;
    }
    let configured_value = descriptor.value.or(descriptor.default);
    let value = configured_value
        .as_ref()
        .map(display_value)
        .unwrap_or_default();
    let choices = descriptor
        .schema
        .get("enum")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .map(|value| PropertyEditorChoice {
            label: display_value(value),
            value: value.to_string(),
        })
        .collect::<Vec<_>>();
    let selected_choice = configured_value
        .as_ref()
        .map(serde_json::Value::to_string)
        .filter(|current| choices.iter().any(|choice| &choice.value == current));
    let label = descriptor.title.as_deref().unwrap_or(&descriptor.key);
    let label = safe_text(label);
    Some(PropertyEditorItem {
        key: descriptor.key,
        label,
        value,
        description: descriptor.description.map(|text| safe_text(&text)),
        choices,
        selected_choice,
    })
}

/// Parses JSON-formatted values and otherwise submits text as a JSON string.
///
/// - `value`: Editor text submitted by the user.
///
/// # Returns
/// Parsed JSON or the original text as a JSON string.
fn parse_value(value: &str) -> serde_json::Value {
    serde_json::from_str(value).unwrap_or_else(|_| serde_json::Value::String(value.to_owned()))
}

/// Formats a safe scalar for upstream property controls.
///
/// - `value`: Provider property value to display.
///
/// # Returns
/// A bounded terminal-safe representation.
fn display_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => safe_text(text),
        _ => safe_text(&value.to_string()),
    }
}

/// Removes terminal control characters and bounds provider-supplied labels.
///
/// - `value`: Untrusted provider property text.
///
/// # Returns
/// Sanitized text bounded to 120 characters.
fn safe_text(value: &str) -> String {
    crate::inspector::sanitize(value)
        .chars()
        .take(120)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hides secret values before a descriptor reaches the upstream editor.
    #[test]
    fn secret_like_properties_are_not_rendered() {
        let descriptor = rpi_model_runtime::ModelPropertyDescriptor {
            key: "api_key".to_owned(),
            title: Some("API key".to_owned()),
            description: None,
            schema: serde_json::json!({"type": "string"}),
            value: Some(serde_json::json!("should-not-render")),
            default: None,
            required: false,
        };

        assert!(property_item(descriptor).is_none());
    }

    /// Keeps schema enums and ordinary values in the upstream editor row.
    #[test]
    fn ordinary_schema_property_preserves_upstream_choice_values() {
        let descriptor = rpi_model_runtime::ModelPropertyDescriptor {
            key: "region".to_owned(),
            title: Some("Region".to_owned()),
            description: None,
            schema: serde_json::json!({
                "type": "string",
                "enum": ["east", "west"]
            }),
            value: Some(serde_json::json!("east")),
            default: None,
            required: true,
        };

        let item = property_item(descriptor).expect("ordinary property is visible");
        assert_eq!(item.value, "east");
        assert_eq!(item.selected_choice.as_deref(), Some("\"east\""));
        assert_eq!(
            item.choices
                .iter()
                .map(|choice| choice.value.as_str())
                .collect::<Vec<_>>(),
            vec!["\"east\"", "\"west\""]
        );
    }

    /// Keeps string enum values distinct from typed scalar enum values.
    #[test]
    fn enum_choice_tokens_preserve_typed_upstream_values() {
        let descriptor = rpi_model_runtime::ModelPropertyDescriptor {
            key: "mode".to_owned(),
            title: None,
            description: None,
            schema: serde_json::json!({"enum": ["true", 1, true, null]}),
            value: Some(serde_json::json!("true")),
            default: None,
            required: false,
        };
        let item = property_item(descriptor).expect("safe enum is visible");

        assert_eq!(item.selected_choice.as_deref(), Some("\"true\""));
        assert_eq!(
            item.choices
                .iter()
                .map(|choice| choice.value.as_str())
                .collect::<Vec<_>>(),
            vec!["\"true\"", "1", "true", "null"]
        );
    }

    /// Hides object editors that contain secret-like nested schema fields.
    #[test]
    fn nested_sensitive_schema_is_not_rendered() {
        let descriptor = rpi_model_runtime::ModelPropertyDescriptor {
            key: "options".to_owned(),
            title: None,
            description: None,
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "safe": {"type": "string"},
                    "nested": {
                        "type": "object",
                        "properties": {
                            "access_token": {"type": "string"}
                        }
                    }
                }
            }),
            value: Some(serde_json::json!({"safe": "ok", "nested": {"access_token": "hidden"}})),
            default: None,
            required: false,
        };

        assert!(property_item(descriptor).is_none());
    }

    /// Parses scalar JSON while treating unquoted editor input as text.
    #[test]
    fn property_input_is_passed_as_typed_json_without_schema_rules() {
        assert_eq!(parse_value("12"), serde_json::json!(12));
        assert_eq!(parse_value("hello"), serde_json::json!("hello"));
    }
}
