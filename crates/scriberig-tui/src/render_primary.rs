//! Renders cached primary view models into terminal text.

use super::render_helpers::inspector_text;
use crate::state::{Focus, TuiState};

/// Renders the selected primary view from cached presentation state.
///
/// # Parameters
/// - `state`: Cached presentation state.
/// - `super`: Parent renderer module.
///
/// # Returns
/// Text representing the selected primary view.
#[allow(clippy::too_many_lines)]
pub(super) fn primary_text(state: &TuiState) -> String {
    match state.primary_view {
        crate::selection::PrimaryView::Overview => inspector_text(state),
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
                    format!(
                        "{}{}{}: {}",
                        if state.focus == Focus::Primary && state.cursor == index {
                            "> "
                        } else {
                            "  "
                        },
                        "  ".repeat(row.depth),
                        if row.transient { "~" } else { "" },
                        row.label
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
                .collect::<Vec<_>>()
                .join("\n"),
            crate::inspector::LoadState::Loading { .. } => "Loading session...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "Session not loaded".to_owned(),
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
