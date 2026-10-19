//! Builds styled conversation rows with ancestry rails and reply highlighting.

use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

use crate::state::{ConversationTarget, TuiState};

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
    Text::from(
        model
            .rows
            .iter()
            .map(|row| {
                let selected = state.conversation_cursor == Some(row.source);
                let replying = reply_anchor.is_some_and(|anchor| row.reply_anchor == Some(anchor));
                let visible = row.depth.min(rail_capacity);
                let first = row.depth.saturating_sub(visible);
                let mut spans = vec![Span::raw(if selected { "> " } else { "  " })];
                if first > 0 {
                    spans.push(Span::raw("…"));
                }
                spans.extend(
                    (first..row.depth)
                        .map(|depth| Span::styled("▏ ", Style::default().fg(depth_color(depth)))),
                );
                let label = if row.depth > visible {
                    format!("{}{}", "  ".repeat(visible), row.label)
                } else {
                    row.label.clone()
                };
                spans.push(Span::styled(
                    label,
                    if replying {
                        Style::default().bg(Color::DarkGray).fg(Color::White)
                    } else {
                        Style::default()
                    },
                ));
                Line::from(spans)
            })
            .collect::<Vec<_>>(),
    )
}

/// Selects a finite rail color from derived branch depth.
///
/// - `depth`: Derived ancestry depth.
///
/// A palette color that cycles for deeper nesting.
/// # Returns
/// The operation result.
fn depth_color(depth: usize) -> Color {
    const PALETTE: [Color; 6] = [
        Color::Cyan,
        Color::Green,
        Color::Yellow,
        Color::Magenta,
        Color::Blue,
        Color::Red,
    ];
    PALETTE[depth % PALETTE.len()]
}

#[cfg(test)]
mod tests {
    use super::{conversation_text, depth_color};
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
        assert_eq!(depth_color(0), Color::Cyan);
        assert_eq!(depth_color(1), Color::Green);
        assert_eq!(depth_color(6), Color::Cyan);
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
}
