//! Projects legacy session turns and their captured tool arguments.

use super::{SessionRow, SessionRowKind};
use scriberig_core::EntityId;
use scriberig_session::{Branch, SessionViews};
use std::collections::BTreeMap;

/// Appends turn and tool rows for one branch.
///
/// # Parameters
/// - `source`: Branch whose turns are projected.
/// - `depth`: Parent branch row depth.
/// - `transient`: Whether the owning branch is still active.
/// - `rows`: Output rows being built.
/// - `tool_invocation_details`: Safe details indexed by invocation identity.
pub(super) fn append(
    source: &Branch,
    depth: usize,
    transient: bool,
    rows: &mut Vec<SessionRow>,
    tool_invocation_details: &mut BTreeMap<EntityId, String>,
) {
    for turn in SessionViews::turns(source) {
        let agent_output = source
            .turns
            .iter()
            .find(|stored| stored.id == turn.id)
            .and_then(|stored| stored.agent.as_ref());
        let label = agent_output.map_or_else(
            || format!("Turn {}", turn.ordinal),
            |agent| {
                format!(
                    "Turn {}: {}",
                    turn.ordinal,
                    crate::inspector::sanitize(&agent.text)
                )
            },
        );
        rows.push(SessionRow {
            reference: turn.id,
            depth: depth + 1,
            label,
            kind: SessionRowKind::Turn,
            candidate: None,
            snapshot: None,
            transient,
        });
        append_turn_tools(
            turn.tool_invocation,
            turn.tool_result,
            depth,
            transient,
            rows,
            tool_invocation_details,
            source,
        );
    }
}

/// Appends a turn's tool references and safe argument detail.
///
/// # Parameters
/// - `invocation`: Stable tool invocation referenced by the turn.
/// - `result`: Stable tool result referenced by the turn.
/// - `depth`: Parent branch row depth.
/// - `transient`: Whether the owning branch is still active.
/// - `rows`: Output rows being built.
/// - `tool_invocation_details`: Safe details indexed by invocation identity.
/// - `source`: Branch containing the invocation records.
fn append_turn_tools(
    invocation: Option<EntityId>,
    result: Option<EntityId>,
    depth: usize,
    transient: bool,
    rows: &mut Vec<SessionRow>,
    tool_invocation_details: &mut BTreeMap<EntityId, String>,
    source: &Branch,
) {
    for reference in [invocation, result].into_iter().flatten() {
        rows.push(SessionRow {
            reference,
            depth: depth + 2,
            label: format!("Tool {reference}"),
            kind: SessionRowKind::Tool,
            candidate: None,
            snapshot: None,
            transient,
        });
    }
    if let Some(reference) = invocation
        && let Some(invocation) = source
            .tool_invocations
            .iter()
            .find(|item| item.id == reference)
        && let Some(detail) = safe_invocation_detail(invocation.presentation_input.as_deref())
    {
        tool_invocation_details.insert(reference, detail);
    }
}

/// Parses and bounds schema-redacted invocation arguments for detailed views.
///
/// # Parameters
/// - `input`: Safe structured arguments captured with the invocation.
///
/// # Returns
/// A sanitized pretty-printed projection capped at 4096 characters.
fn safe_invocation_detail(input: Option<&[u8]>) -> Option<String> {
    let value = serde_json::from_slice::<serde_json::Value>(input?).ok()?;
    let text = serde_json::to_string_pretty(&value).ok()?;
    Some(
        text.chars()
            .filter(|character| *character == '\n' || !character.is_control())
            .take(4096)
            .collect(),
    )
}
