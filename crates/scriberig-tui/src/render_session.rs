//! Builds styled session activity rows from cached TUI state.

use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

use crate::{inspector::LoadState, state::TuiState};

/// Renders session activity with visible rails and finite depth colors.
///
/// - `state`: Cached session and presentation state.
/// - `available_width`: Width available for the session rows.
///
/// Session activity text with hierarchy preserved independently of color.
/// # Returns
/// The operation result.
pub(super) fn session_text(state: &TuiState, available_width: usize) -> Text<'static> {
    let LoadState::Ready(model) = &state.session else {
        return Text::from(super::render_primary::primary_text(state));
    };
    let mut lines = model
        .rows
        .iter()
        .enumerate()
        .flat_map(|(index, row)| {
            let label = super::render_primary::session_row_label(state, row);
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
            let mut row_lines = label.lines();
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
    lines.extend(state.agent_output_observations.iter().map(|output| {
        Line::from(format!(
            "[stream, non-canonical] {}",
            crate::inspector::sanitize(output)
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

/// Selects a stable terminal color for one nesting depth.
///
/// - `depth`: Zero-based ancestor depth.
///
/// A color from the finite hierarchy palette.
/// # Returns
/// The operation result.
fn depth_color(depth: usize) -> Color {
    const PALETTE: [Color; 6] = [
        Color::Cyan,
        Color::Green,
        Color::Yellow,
        Color::Magenta,
        Color::Blue,
        Color::LightCyan,
    ];
    PALETTE[depth % PALETTE.len()]
}
