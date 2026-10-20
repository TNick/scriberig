//! Builds styled session activity rows from cached TUI state.

use ratatui::{
    style::Style,
    text::{Line, Span, Text},
};
use std::ops::Range;

use crate::{inspector::LoadState, render::render_helpers::depth_color, state::TuiState};

/// Maximum untrusted row-label characters presented in one terminal frame.
const MAX_SESSION_LABEL_CHARS: usize = 512;

/// Renders session activity with visible rails and finite depth colors.
///
/// - `state`: Cached session and presentation state.
/// - `available_width`: Width available for the session rows.
/// - `available_rows`: Number of rows available for session presentation.
///
/// Session activity text with hierarchy preserved independently of color.
/// # Returns
/// The operation result.
pub(super) fn session_text(
    state: &TuiState,
    available_width: usize,
    available_rows: usize,
) -> Text<'static> {
    let LoadState::Ready(model) = &state.session else {
        return Text::from(super::render_primary::primary_text(state));
    };
    let visible_rows = visible_session_rows(
        model.rows.len(),
        state.cursor,
        state.focus == crate::state::Focus::Primary,
        available_rows,
    );
    let mut lines = model
        .rows
        .iter()
        .enumerate()
        .skip(visible_rows.start)
        .take(visible_rows.len())
        .flat_map(|(index, row)| {
            let label = super::render_primary::session_row_label(state, row);
            let label = scriberig_tui_kit::ExternalText::new(&bounded_label(&label));
            let mut spans = vec![Span::raw(
                if state.focus == crate::state::Focus::Primary && state.cursor == index {
                    "> "
                } else {
                    "  "
                },
            )];
            let rail_capacity = available_width.saturating_sub(12) / 2;
            let visible_rails = row.depth.min(rail_capacity);
            let first_visible_depth = row.depth.saturating_sub(visible_rails);
            if first_visible_depth > 0 {
                spans.push(Span::raw("…"));
            }
            spans.extend(
                (first_visible_depth..row.depth)
                    .map(|depth| Span::styled("▏ ", Style::default().fg(depth_color(depth)))),
            );
            if row.transient {
                spans.push(Span::raw("~ "));
            }
            let mut row_lines = label.as_str().lines();
            let first = row_lines.next().unwrap_or_default();
            spans.push(Span::raw(first.to_owned()));
            let mut rendered = vec![Line::from(spans)];
            let continuation_indent = format!(
                "  {}{}",
                if first_visible_depth > 0 { "…" } else { "" },
                "  ".repeat(visible_rails)
            );
            rendered
                .extend(row_lines.map(|line| Line::from(format!("{continuation_indent}{line}"))));
            rendered
        })
        .collect::<Vec<_>>();
    lines.extend(state.transient_agent_outputs.iter().map(|output| {
        Line::from(format!(
            "[stream · {}] {}",
            output.operation,
            crate::inspector::sanitize(&output.text)
        ))
    }));
    lines.extend(model.execution.history.iter().map(|change| {
        let label = match change.kind {
            scriberig_session::AgentActivityKind::ModelSelectionChanged => "Model selection",
            scriberig_session::AgentActivityKind::ThinkingLevelChanged => "Thinking selection",
            _ => "Execution selection",
        };
        Line::from(format!(
            "[{label} · revision {}] {}",
            change.revision,
            crate::inspector::sanitize(&change.summary)
        ))
    }));
    if state.reasoning_visibility == crate::state::ReasoningVisibility::Shown {
        lines.extend(
            state
                .transient_agent_activities
                .iter()
                .map(|activity| {
                    let label = if activity.activity_kind
                        == scriberig_session::AgentActivityKind::ReasoningSummary
                    {
                        "Reasoning summary"
                    } else {
                        "Live activity"
                    };
                    Line::from(format!(
                        "[{label}] {}: {} ({:?})",
                        activity.branch,
                        crate::inspector::sanitize(&activity.summary),
                        activity.state
                    ))
                })
                .collect::<Vec<_>>(),
        );
    }
    Text::from(lines)
}

/// Bounds a row label before terminal sanitization allocates its display form.
///
/// - `label`: User or runtime supplied session row text.
///
/// A bounded prefix with a truncation marker when content was clipped.
/// # Returns
/// The bounded safe label.
fn bounded_label(label: &str) -> String {
    let mut characters = label.chars();
    let mut bounded = characters
        .by_ref()
        .take(MAX_SESSION_LABEL_CHARS)
        .collect::<String>();
    if characters.next().is_some() {
        bounded.push('…');
    }
    bounded
}

/// Selects the session rows that fit the viewport around the active row.
///
/// - `row_count`: Number of rows in the complete semantic session tree.
/// - `cursor`: Selected row index.
/// - `focused`: Whether the session tree owns keyboard focus.
/// - `available_rows`: Number of rows available for presentation.
///
/// A bounded row range that keeps the selected row visible when focused.
/// # Returns
/// The visible row indices.
fn visible_session_rows(
    row_count: usize,
    cursor: usize,
    focused: bool,
    available_rows: usize,
) -> Range<usize> {
    let visible_count = row_count.min(available_rows.max(1));
    let selected = if focused {
        cursor.min(row_count.saturating_sub(1))
    } else {
        0
    };
    let start = selected.saturating_add(1).saturating_sub(visible_count);
    start..start.saturating_add(visible_count).min(row_count)
}

#[cfg(test)]
mod tests {
    use super::session_text;
    use crate::{inspector::LoadState, state::TuiState, view::session::SessionRow};

    /// Verifies large session trees render only rows that fit the viewport.
    #[test]
    fn large_session_tree_renders_a_bounded_selected_window() {
        let rows = (0..10_000)
            .map(|index| SessionRow {
                reference: scriberig_core::EntityId::from_bytes(
                    [u8::try_from(index % 256).expect("bounded entity byte"); 16],
                ),
                depth: index % 8,
                label: if index == 9_999 {
                    "x".repeat(100_000)
                } else {
                    format!("branch-{index}")
                },
                kind: crate::view::session::SessionRowKind::Branch,
                candidate: None,
                snapshot: None,
                transient: false,
            })
            .collect();
        let mut state = TuiState::default();
        state.session = LoadState::Ready(crate::view::session::SessionModel {
            rows,
            ..crate::view::session::SessionModel::default()
        });
        state.cursor = 9_999;
        state.focus = crate::state::Focus::Primary;

        let rendered = session_text(&state, 80, 12);
        let text = rendered
            .lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        assert_eq!(text.len(), 12);
        assert!(text.last().is_some_and(|row| row.ends_with('…')));
        assert!(text.last().is_some_and(|row| row.chars().count() <= 540));
        assert!(!text.iter().any(|row| row.contains("branch-0")));
    }
}
