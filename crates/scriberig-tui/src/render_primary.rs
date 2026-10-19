//! Renders cached primary view models into terminal text.

use super::render_helpers::inspector_text;
use crate::state::{Focus, TuiState};
/// Renders the selected primary view from cached presentation state.
///
/// - `state`: Cached presentation state.
/// - `super`: Parent renderer module.
///
/// Text representing the selected primary view.
#[allow(clippy::too_many_lines)]
/// # Returns
/// The operation result.
pub(crate) fn primary_text(state: &TuiState) -> String {
    match state.primary_view {
        crate::selection::PrimaryView::Conversation => state.conversation.as_ref().map_or_else(
            || "No conversation yet. Type a message to start a session.".to_owned(),
            |model| {
                model
                    .rows
                    .iter()
                    .map(|row| format!("{}{}", "  ".repeat(row.depth), row.label))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
        ),
        crate::selection::PrimaryView::Overview => inspector_text(state),
        crate::selection::PrimaryView::Explorer => {
            crate::render::render_explorer::explorer_text(state)
        }
        crate::selection::PrimaryView::History => match &state.history {
            crate::inspector::LoadState::Ready(model) => format!(
                "History @ {}\n{}",
                model.snapshot,
                model
                    .rows
                    .iter()
                    .enumerate()
                    .map(|(index, row)| {
                        let base = row.base.to_string();
                        let result = row.result.to_string();
                        let change =
                            row.source_change_id
                                .as_ref()
                                .map_or_else(String::new, |change| {
                                    format!(" [C1:{}]", change.chars().take(8).collect::<String>())
                                });
                        format!(
                            "{}{}: {} -> {} (+{}){}",
                            if state.focus == Focus::Primary && state.cursor == index {
                                "> "
                            } else {
                                "  "
                            },
                            index + 1,
                            &base[..16],
                            &result[..16],
                            row.entities.len(),
                            change
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            crate::inspector::LoadState::Loading { .. } => "Loading history...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "History not loaded".to_owned(),
        },
        crate::selection::PrimaryView::Find => match &state.find {
            crate::inspector::LoadState::Ready(model) => format!(
                "Find: {}\n{}",
                model.query,
                model
                    .rows
                    .iter()
                    .enumerate()
                    .map(|(index, row)| format!(
                        "{}{} {}{}",
                        if state.focus == Focus::Primary && state.cursor == index {
                            "> "
                        } else {
                            "  "
                        },
                        row.entity,
                        row.entity_type.as_deref().unwrap_or("unknown"),
                        row.preview
                            .as_deref()
                            .map_or(String::new(), |value| format!(" — {value}"))
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            crate::inspector::LoadState::Loading { .. } => "Searching...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "Find not run".to_owned(),
        },
        crate::selection::PrimaryView::Diff => match &state.diff {
            crate::inspector::LoadState::Ready(model) => format!(
                "Diff\nleft/base: {}\nright/target: {}\n{}",
                model.left,
                model.right,
                model
                    .rows
                    .iter()
                    .enumerate()
                    .map(|(index, row)| {
                        format!(
                            "{}{}: {:?} -> {:?}",
                            if state.focus == Focus::Primary && state.cursor == index {
                                "> "
                            } else {
                                "  "
                            },
                            row.entity,
                            row.before,
                            row.after
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            crate::inspector::LoadState::Loading { .. } => "Loading diff...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "Diff not loaded".to_owned(),
        },
        crate::selection::PrimaryView::Projection => match &state.projection {
            crate::inspector::LoadState::Ready(model) => format!(
                "Projection {}\nsource: {}\nstyle: {}\nbase: {}\n{}",
                model.projection.as_deref().unwrap_or("unknown"),
                model.source_entity,
                model.style,
                model.base_snapshot,
                model.content
            ),
            crate::inspector::LoadState::Loading { .. } => "Rendering projection...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "Projection not loaded".to_owned(),
        },
        crate::selection::PrimaryView::Changes => match &state.review {
            crate::inspector::LoadState::Ready(model) => {
                let fields = model.change.splitn(3, '|').collect::<Vec<_>>();
                let selected = model.conflicts.get(state.cursor);
                let review_diff = selected.and_then(|value| {
                    let (scope_and_path, rest) = value.split_once("add | ")?;
                    let prefix = scope_and_path
                        .trim_end_matches(": ")
                        .replace("; ", "\npath: ");
                    Some(format!("{prefix}\n{}", rest.replace(" | ", "\n")))
                });
                let detail = if let Some(diff) = review_diff {
                    diff
                } else if fields.len() == 3 && fields[0] == "project-edit" {
                    format!("path: src/lib.rs\nold: left + right\nnew: {}", fields[2])
                } else if let Some(change) = selected {
                    format!(
                        "candidate: {}\nfile {}/{}\n{}",
                        model.change,
                        state.cursor + 1,
                        model.conflicts.len(),
                        change
                    )
                } else {
                    format!("change: {}", model.change)
                };
                format!(
                    "Change review\nbase: {}\nvalidation: {:?}\nprofile: {}\n{}\nUp/Down select file | Enter to confirm",
                    model.base_snapshot,
                    model.validation,
                    model.validation_profile.as_deref().unwrap_or("unspecified"),
                    detail,
                )
            }
            crate::inspector::LoadState::Loading { .. } => "Preparing change review...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "No change review".to_owned(),
        },
        crate::selection::PrimaryView::Session => match &state.session {
            crate::inspector::LoadState::Ready(model) => model
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    let label = session_row_label(state, row);
                    format!(
                        "{}{}{}{}",
                        if state.focus == Focus::Primary && state.cursor == index {
                            "> "
                        } else {
                            "  "
                        },
                        "▏ ".repeat(row.depth),
                        if row.transient { "~ " } else { "" },
                        label
                    )
                })
                .collect::<Vec<_>>()
                .into_iter()
                .chain(state.agent_output_observations.iter().map(|output| {
                    format!(
                        "[stream, non-canonical] {}",
                        crate::inspector::sanitize(output)
                    )
                }))
                .chain(
                    (state.reasoning_visibility == crate::state::ReasoningVisibility::Shown)
                        .then_some(state.transient_agent_activities.as_slice())
                        .into_iter()
                        .flatten()
                        .map(|activity| {
                            let label = if activity.activity_kind
                                == scriberig_session::AgentActivityKind::ReasoningSummary
                            {
                                "Reasoning summary"
                            } else {
                                "Live activity"
                            };
                            format!(
                                "[{label}] {}: {} ({:?})",
                                activity.branch,
                                crate::inspector::sanitize(&activity.summary),
                                activity.state
                            )
                        }),
                )
                .collect::<Vec<_>>()
                .join("\n"),
            crate::inspector::LoadState::Loading { .. } => "Loading session...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => {
                if state
                    .session_composer
                    .as_ref()
                    .is_some_and(|composer| composer.branch.is_none())
                {
                    "New agent session".to_owned()
                } else {
                    "Session not loaded".to_owned()
                }
            }
        },
        crate::selection::PrimaryView::Operations => {
            if state.operations.is_empty() {
                "No observed operations".to_owned()
            } else {
                state
                    .operations
                    .iter()
                    .enumerate()
                    .map(|(index, operation)| {
                        format!(
                            "{}{} [{:?}] {}",
                            if state.focus == Focus::Primary && state.cursor == index {
                                "> "
                            } else {
                                "  "
                            },
                            operation.id,
                            operation.status,
                            operation.kind
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }
}

/// Resolves the current client-local activity presentation for one row.
///
/// - `state`: Cached TUI state with local visibility and loaded detail.
/// - `row`: Session activity row to render.
///
/// A safe row label or bounded detail presentation.
/// # Returns
/// The operation result.
pub(super) fn session_row_label(
    state: &TuiState,
    row: &crate::view::session::SessionRow,
) -> String {
    if row.kind == crate::view::session::SessionRowKind::ToolInvocation
        && state.tool_invocation_visibility
            == crate::state::ToolInvocationVisibility::StructuredParameters
        && let crate::inspector::LoadState::Ready(model) = &state.session
        && let Some(detail) = model.tool_invocation_details.get(&row.reference)
    {
        return format!("{}\n  Arguments:\n{}", row.label, detail);
    }
    if row.kind == crate::view::session::SessionRowKind::ToolInvocation
        && state.tool_invocation_visibility
            == crate::state::ToolInvocationVisibility::CompactTruncated
        && let crate::inspector::LoadState::Ready(model) = &state.session
        && let Some(detail) = model.tool_invocation_details.get(&row.reference)
    {
        return format!("{}{}", row.label, compact_tool_arguments(detail));
    }
    if row.kind == crate::view::session::SessionRowKind::ToolInvocation
        && state.tool_invocation_visibility == crate::state::ToolInvocationVisibility::Full
        && let crate::inspector::LoadState::Ready(model) = &state.session
        && let Some(detail) = model.tool_invocation_details.get(&row.reference)
    {
        return format!("{}\n  Safe arguments (bounded):\n{}", row.label, detail);
    }
    if row.kind == crate::view::session::SessionRowKind::ToolInvocation
        && state.tool_invocation_visibility == crate::state::ToolInvocationVisibility::Hidden
    {
        return "Tool invocation hidden".to_owned();
    }
    if row.kind == crate::view::session::SessionRowKind::ModelInvocation
        && state.prompt_visibility == crate::state::PromptVisibility::Shown
        && let Some(detail) = state.model_request_details.get(&row.reference)
    {
        return format!("{}{}", row.label, format_model_request_detail(detail));
    }
    if row.kind != crate::view::session::SessionRowKind::ToolResult {
        return row.label.clone();
    }
    match state.tool_output_visibility {
        crate::state::ToolOutputVisibility::Hidden => "Tool result hidden".to_owned(),
        crate::state::ToolOutputVisibility::Summary => row.label.clone(),
        crate::state::ToolOutputVisibility::Full => {
            state.tool_result_details.get(&row.reference).map_or_else(
                || "Loading tool result detail…".to_owned(),
                |detail| {
                    let content = serde_json::to_string_pretty(&detail.detail)
                        .unwrap_or_else(|_| "{}".to_owned());
                    format!(
                        "Tool result detail{}: {content}",
                        if detail.truncated { " (truncated)" } else { "" }
                    )
                },
            )
        }
    }
}

#[path = "render_primary_details.rs"]
mod details;
use details::{compact_tool_arguments, format_model_request_detail};
