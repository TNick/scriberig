//! Builds styled conversation rows with ancestry rails and reply highlighting.

use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

use crate::{
    render::render_helpers::depth_color,
    state::{ConversationTarget, TuiState},
};
use scriberig_tui_kit::{ExternalText, render_markdown};

/// Renders conversation rows with independently styled ancestry and reply target.
///
/// - `state`: Cached conversation and navigation state.
/// - `available_width`: Width available for the rows.
///
/// Styled conversation rows with stable source identities controlling selection.
/// # Returns
/// The operation result.
pub(super) fn conversation_text(state: &TuiState, available_width: usize) -> Text<'static> {
    let Some(model) = state.conversation.as_ref() else {
        return Text::from("No conversation yet. Type a message to start a session.");
    };
    let reply_anchor = match state.conversation_composer.target {
        ConversationTarget::Existing { reply_anchor, .. } => Some(reply_anchor),
        ConversationTarget::NewSession => None,
    };
    let rail_capacity = (available_width.saturating_sub(8) / 2).max(1);
    let lines = model
        .rows
        .iter()
        .flat_map(|row| {
            let selected = state.conversation_cursor == Some(row.source);
            let replying = reply_anchor.is_some_and(|anchor| row.reply_anchor == Some(anchor));
            let visible = row.depth.min(rail_capacity);
            let first = row.depth.saturating_sub(visible);
            let rail_text = if first > 0 { "…" } else { "" };
            let rails = (first..row.depth)
                .map(|depth| Span::styled("▏ ", Style::default().fg(depth_color(depth))))
                .collect::<Vec<_>>();
            let content_width = available_width
                .saturating_sub(2 + usize::from(first > 0) + visible.saturating_mul(2))
                .max(1);
            let labels = if row.kind == crate::view::conversation::ConversationRowKind::Message {
                render_markdown(&row.label, content_width)
            } else {
                vec![ExternalText::new(&row.label)]
            };
            let indentation = if row.depth > visible {
                "  ".repeat(visible)
            } else {
                String::new()
            };
            labels
                .into_iter()
                .enumerate()
                .map(move |(line_index, label)| {
                    let mut spans = vec![Span::raw(if selected && line_index == 0 {
                        "> "
                    } else {
                        "  "
                    })];
                    if line_index == 0 {
                        if !rail_text.is_empty() {
                            spans.push(Span::raw(rail_text));
                        }
                        spans.extend(rails.clone());
                    } else {
                        spans.push(Span::raw("  ".repeat(visible + usize::from(first > 0))));
                    }
                    spans.push(Span::styled(
                        format!("{indentation}{}", label.as_str()),
                        if replying {
                            Style::default().bg(Color::DarkGray).fg(Color::White)
                        } else {
                            Style::default()
                        },
                    ));
                    Line::from(spans)
                })
        })
        .collect::<Vec<_>>();
    Text::from(lines)
}

/// Selects a finite rail color from derived branch depth.
///
/// - `depth`: Derived ancestry depth.
///
/// A palette color that cycles for deeper nesting.
/// # Returns
/// The operation result.
#[cfg(test)]
mod tests {
    use super::conversation_text;
    use crate::render::render_helpers::depth_color;
    use crate::{
        state::TuiState,
        view::conversation::{
            ConversationModel, ConversationRow, ConversationRowKind, ConversationSource,
        },
    };
    use ratatui::{style::Color, text::Text};
    use scriberig_core::EntityId;

    /// Verifies depth colors cycle through the finite ancestry palette.
    #[test]
    fn depth_palette_cycles_after_all_colors() {
        assert_eq!(depth_color(0), Color::Indexed(39));
        assert_eq!(depth_color(1), Color::Indexed(40));
        assert_eq!(depth_color(6), Color::Indexed(39));
    }

    /// Verifies narrow rows keep a local rail and mark omitted ancestors.
    #[test]
    fn narrow_rows_compact_ancestors_with_an_ellipsis() {
        let state = TuiState {
            conversation: Some(ConversationModel {
                rows: vec![ConversationRow {
                    source: ConversationSource::Branch(EntityId::from_bytes([1; 16])),
                    reply_anchor: None,
                    depth: 3,
                    label: "nested branch".to_owned(),
                    kind: ConversationRowKind::Branch,
                }],
                coordinator_cursor: None,
            }),
            ..TuiState::default()
        };

        let text = conversation_text(&state, 10);
        let rendered = text.lines[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(rendered.contains('…'));
        assert!(rendered.contains("▏ "));
        assert!(rendered.contains("nested branch"));
    }

    /// Verifies the reply target uses a background separate from rail colors.
    #[test]
    fn reply_target_has_background_highlight() {
        use crate::state::ConversationTarget;
        let branch = EntityId::from_bytes([2; 16]);
        let turn = EntityId::from_bytes([3; 16]);
        let anchor = scriberig_session::ContinuationAnchor::AssistantOutput { branch, turn };
        let source = ConversationSource::Activity(turn);
        let mut state = TuiState::default();
        state.conversation_composer.target = ConversationTarget::Existing {
            session: EntityId::from_bytes([4; 16]),
            reply_anchor: anchor,
        };
        state.conversation = Some(ConversationModel {
            rows: vec![ConversationRow {
                source,
                reply_anchor: Some(anchor),
                depth: 1,
                label: "reply target".to_owned(),
                kind: ConversationRowKind::Message,
            }],
            coordinator_cursor: None,
        });

        let text: Text<'static> = conversation_text(&state, 80);
        let label = text.lines[0]
            .spans
            .iter()
            .find(|span| span.content == "reply target")
            .expect("reply label");

        assert_eq!(label.style.bg, Some(Color::DarkGray));
    }

    /// Verifies message Markdown remains structured safe text in the TUI.
    #[test]
    fn message_markdown_uses_safe_terminal_rendering() {
        let state = TuiState {
            conversation: Some(ConversationModel {
                rows: vec![ConversationRow {
                    source: ConversationSource::Activity(EntityId::from_bytes([8; 16])),
                    reply_anchor: None,
                    depth: 0,
                    label: "# heading\n\nunsafe \u{1b}[2J".to_owned(),
                    kind: ConversationRowKind::Message,
                }],
                coordinator_cursor: None,
            }),
            ..TuiState::default()
        };

        let rendered = conversation_text(&state, 40);
        let plain = rendered
            .lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(plain.contains("heading"));
        assert!(plain.contains("\\u{1b}[2J"));
        assert!(!plain.contains('\u{1b}'));
    }
}
