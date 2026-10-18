//! Projects ordered durable execution activity into terminal session rows.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::{SessionRow, SessionRowKind};
use scriberig_core::EntityId;
use scriberig_session::{Branch, ExecutionActivity};

/// Appends the branch's durable execution sequence when one is available.
///
/// # Parameters
/// - `branch`: Branch containing ordered execution activity.
/// - `depth`: Branch nesting depth in the session tree.
/// - `transient`: Whether the branch is still active.
/// - `rows`: Output rows receiving projected activity.
/// - `tool_invocation_details`: Safe details indexed by invocation identity.
///
/// # Returns
/// Whether the branch had durable activity to project.
pub(super) fn append(
    branch: &Branch,
    depth: usize,
    transient: bool,
    rows: &mut Vec<SessionRow>,
    tool_invocation_details: &mut BTreeMap<EntityId, String>,
) -> bool {
    if branch.activity.is_empty() {
        return false;
    }
    let mut invocation_depths = BTreeMap::new();
    let mut tool_depths = BTreeMap::new();
    let mut current_model_invocation = None;
    for activity in &branch.activity {
        match activity {
            ExecutionActivity::HumanInput(id) => append_human(branch, *id, depth, transient, rows),
            ExecutionActivity::ModelInvocation(id) => {
                let Some(invocation) = branch
                    .model_invocations
                    .iter()
                    .find(|invocation| invocation.id == *id)
                else {
                    continue;
                };
                let row_depth = depth + 1;
                invocation_depths.insert(*id, row_depth);
                current_model_invocation = Some(*id);
                append_row(
                    *id,
                    row_depth,
                    format_model_invocation(invocation),
                    SessionRowKind::ModelInvocation,
                    transient,
                    rows,
                );
            }
            ExecutionActivity::ModelOutput(id) => {
                let Some(output) = branch.model_outputs.iter().find(|output| output.id == *id)
                else {
                    continue;
                };
                let row_depth = invocation_depths
                    .get(&output.invocation)
                    .map_or(depth + 1, |parent| parent + 1);
                append_row(
                    *id,
                    row_depth,
                    output.text.as_deref().map_or_else(
                        || "Assistant output".to_owned(),
                        |text| format!("Assistant: {}", crate::inspector::sanitize(text)),
                    ),
                    SessionRowKind::ModelOutput,
                    transient,
                    rows,
                );
            }
            ExecutionActivity::AgentOutput(id) => append_agent_output(
                branch,
                *id,
                depth,
                transient,
                current_model_invocation.and_then(|parent| invocation_depths.get(&parent).copied()),
                rows,
            ),
            ExecutionActivity::ToolInvocation(id) => {
                let row_depth = current_model_invocation
                    .and_then(|parent| invocation_depths.get(&parent).copied())
                    .map_or(depth + 1, |parent| parent + 1);
                tool_depths.insert(*id, row_depth);
                append_tool_invocation(
                    branch,
                    *id,
                    row_depth,
                    transient,
                    rows,
                    tool_invocation_details,
                );
            }
            ExecutionActivity::ToolResult(id) => {
                let Some(result) = branch.tool_results.iter().find(|result| result.id == *id)
                else {
                    continue;
                };
                let row_depth = tool_depths
                    .get(&result.invocation)
                    .map_or(depth + 1, |parent| parent + 1);
                append_row(
                    *id,
                    row_depth,
                    format_tool_result(result),
                    SessionRowKind::ToolResult,
                    transient,
                    rows,
                );
            }
            ExecutionActivity::ChildBranch(_)
            | ExecutionActivity::ChildBranchSpawned { .. }
            | ExecutionActivity::BranchResult(_) => {}
        }
    }
    true
}

/// Appends one tool invocation and caches bounded schema-redacted arguments.
///
/// # Parameters
/// - `branch`: Branch containing the invocation.
/// - `id`: Stable invocation identity.
/// - `row_depth`: Activity depth for the rendered row.
/// - `transient`: Whether the branch is still active.
/// - `rows`: Output rows receiving the invocation.
/// - `tool_invocation_details`: Safe detail cache indexed by invocation id.
fn append_tool_invocation(
    branch: &Branch,
    id: EntityId,
    row_depth: usize,
    transient: bool,
    rows: &mut Vec<SessionRow>,
    tool_invocation_details: &mut BTreeMap<EntityId, String>,
) {
    let Some(invocation) = branch
        .tool_invocations
        .iter()
        .find(|invocation| invocation.id == id)
    else {
        return;
    };
    if let Some(detail) = invocation
        .presentation_input
        .as_deref()
        .and_then(|input| serde_json::from_slice::<serde_json::Value>(input).ok())
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
    {
        tool_invocation_details.insert(
            id,
            detail
                .chars()
                .filter(|character| *character == '\n' || !character.is_control())
                .take(4096)
                .collect(),
        );
    }
    append_row(
        id,
        row_depth,
        invocation
            .presentation_label
            .clone()
            .unwrap_or_else(|| format!("Tool: {}", crate::inspector::sanitize(&invocation.name))),
        SessionRowKind::ToolInvocation,
        transient,
        rows,
    );
}

/// Appends a human-input row without exposing prompt text by default.
///
/// # Parameters
/// - `branch`: Branch containing the input turn.
/// - `id`: Stable input turn identity.
/// - `depth`: Branch nesting depth.
/// - `transient`: Whether the branch is still active.
/// - `rows`: Output rows receiving the input.
fn append_human(
    branch: &Branch,
    id: EntityId,
    depth: usize,
    transient: bool,
    rows: &mut Vec<SessionRow>,
) {
    let Some(turn) = branch.turns.iter().find(|turn| turn.id == id) else {
        return;
    };
    if turn.human.is_some() {
        append_row(
            id,
            depth + 1,
            format!("Human input · turn {}", turn.ordinal),
            SessionRowKind::Turn,
            transient,
            rows,
        );
    }
}

/// Appends an assistant output row from the legacy turn representation.
///
/// # Parameters
/// - `branch`: Branch containing the output turn.
/// - `id`: Stable output turn identity.
/// - `depth`: Branch nesting depth.
/// - `transient`: Whether the branch is still active.
/// - `invocation_depth`: Parent model invocation depth when known.
/// - `rows`: Output rows receiving the assistant output.
fn append_agent_output(
    branch: &Branch,
    id: EntityId,
    depth: usize,
    transient: bool,
    invocation_depth: Option<usize>,
    rows: &mut Vec<SessionRow>,
) {
    let Some(turn) = branch.turns.iter().find(|turn| turn.id == id) else {
        return;
    };
    let Some(output) = turn.agent.as_ref() else {
        return;
    };
    let row_depth = invocation_depth.map_or(depth + 1, |parent| parent + 1);
    append_row(
        id,
        row_depth,
        format!("Assistant: {}", crate::inspector::sanitize(&output.text)),
        SessionRowKind::ModelOutput,
        transient,
        rows,
    );
}

/// Formats the safe model identity and lifecycle summary.
///
/// # Parameters
/// - `invocation`: Durable model invocation to display.
///
/// # Returns
/// A concise model and thinking-level label.
fn format_model_invocation(invocation: &scriberig_session::ModelInvocation) -> String {
    let mut label = invocation.model.as_deref().map_or_else(
        || "Model request".to_owned(),
        |model| format!("Model: {model}"),
    );
    if let Some(thinking) = invocation.thinking_level.as_deref() {
        let _ = write!(label, " · Thinking {thinking}");
    }
    if invocation.output.is_none() {
        label.push_str(" · running");
    }
    label
}

/// Formats only the safe summary from an allowlisted result preview.
///
/// # Parameters
/// - `result`: Durable tool result with optional bounded presentation preview.
///
/// # Returns
/// A safe, bounded result label without raw output fields.
fn format_tool_result(result: &scriberig_session::ToolResult) -> String {
    let summary = result
        .presentation_preview
        .as_deref()
        .and_then(|preview| serde_json::from_slice::<serde_json::Value>(preview).ok())
        .and_then(|preview| preview.get("summary")?.as_str().map(str::to_owned));
    let Some(summary) = summary else {
        return if result.failed {
            "Tool result: failed".to_owned()
        } else {
            "Tool result: completed".to_owned()
        };
    };
    let bounded = crate::inspector::sanitize(&summary)
        .chars()
        .take(256)
        .collect::<String>();
    if result.failed {
        format!("Tool result failed: {bounded}")
    } else {
        format!("Tool result: {bounded}")
    }
}

/// Adds one stable semantic activity row.
///
/// # Parameters
/// - `reference`: Stable activity identity.
/// - `depth`: Derived terminal nesting depth.
/// - `label`: Safe display text.
/// - `kind`: Activity classification.
/// - `transient`: Whether the activity is in a live branch.
/// - `rows`: Output rows receiving the activity.
fn append_row(
    reference: EntityId,
    depth: usize,
    label: String,
    kind: SessionRowKind,
    transient: bool,
    rows: &mut Vec<SessionRow>,
) {
    rows.push(SessionRow {
        reference,
        depth,
        label,
        kind,
        candidate: None,
        snapshot: None,
        transient,
    });
}
