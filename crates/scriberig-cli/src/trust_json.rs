//! Renders trust inspection and denial values as structured JSON.

use scriberig_app::{CapabilityDenial, InspectResult, SourceKind, ValidationState};

use crate::trust::InspectDepth;

/// Renders an inspection result as a stable JSON object.
///
/// - `result`: Shared application inspection result.
/// - `depth`: Compact or deep trust view.
///
/// A valid JSON object containing the requested fields.
#[must_use]
/// # Returns
/// The operation result.
pub fn render_inspection_json(result: &InspectResult, depth: InspectDepth) -> String {
    let mut fields = vec![format!(
        "\"entity\":{}",
        json_string(&result.entity.to_string())
    )];
    if let Some(snapshot) = result.snapshot {
        fields.push(format!(
            "\"snapshot\":{}",
            json_string(&snapshot.to_string())
        ));
    }
    if let Some(revision) = result.revision {
        fields.push(format!(
            "\"revision\":{}",
            json_string(&revision.to_string())
        ));
    }
    if let Some(trust) = &result.trust {
        fields.push(format!(
            "\"source\":{}",
            json_string(source_label(&trust.source))
        ));
        fields.push(format!(
            "\"validation\":{}",
            json_string(validation_label(&trust.validation))
        ));
        if let Some(accepted_via) = &trust.accepted_via {
            fields.push(format!("\"accepted_via\":{}", json_string(accepted_via)));
        }
        if depth == InspectDepth::Deep {
            let provenance = trust
                .provenance
                .iter()
                .map(|item| json_string(item))
                .collect::<Vec<_>>()
                .join(",");
            fields.push(format!("\"provenance\":[{provenance}]"));
        }
    }
    format!("{{{}}}", fields.join(","))
}

/// Renders a capability denial as a structured JSON object.
///
/// - `denial`: Shared application denial details.
///
/// Valid JSON containing every denial field without restricted content.
#[must_use]
/// # Returns
/// The operation result.
pub fn render_denial_json(denial: &CapabilityDenial) -> String {
    let alternatives = denial
        .alternatives
        .iter()
        .map(|item| json_string(item))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"action\":{},\"scope\":{},\"reason\":{},\"alternatives\":[{}],\"approval_may_grant\":{}}}",
        json_string(&denial.action),
        json_string(&denial.scope),
        json_string(&denial.reason),
        alternatives,
        denial.approval_may_grant
    )
}

/// Returns the stable display label for a source kind.
///
/// - `source`: Source kind to label.
///
/// Stable machine-readable source label.
/// # Returns
/// The operation result.
fn source_label(source: &SourceKind) -> &'static str {
    match source {
        SourceKind::Authored => "authored",
        SourceKind::Observed => "observed",
        SourceKind::Imported => "imported",
        SourceKind::Derived => "derived",
        SourceKind::ModelGenerated => "model_generated",
    }
}

/// Returns the stable display label for a validation state.
///
/// - `validation`: Validation state to label.
///
/// Stable machine-readable validation label.
/// # Returns
/// The operation result.
fn validation_label(validation: &ValidationState) -> &'static str {
    match validation {
        ValidationState::Current => "current",
        ValidationState::Stale => "stale",
        ValidationState::Failed => "failed",
        ValidationState::Unknown => "unknown",
    }
}

/// Escapes a string for inclusion in a JSON string literal.
///
/// - `value`: String content to escape.
///
/// A quoted JSON string literal.
/// # Returns
/// The operation result.
fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                use std::fmt::Write;
                let _ = write!(escaped, "\\u{:04x}", character as u32);
            }
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}
