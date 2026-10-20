//! Exercises the Scriberig terminal presentation facade with fixed fixtures.

use rpi_tui::Component;
use scriberig_tui_kit::{
    ClipboardPolicy, Composer, CursorMotion, ExternalText, PaletteColor, SelectionOption,
    StableSelector, TerminalPolicy, TextBlock, Tone, display_width, render_markdown, style_for,
};

/// Groups tests for text sanitization and display width.
mod text_tests {
    use super::{
        Composer, ExternalText, SelectionOption, StableSelector, TextBlock, display_width,
        render_markdown,
    };

    /// Verifies unsafe terminal control characters become visible data.
    #[test]
    fn external_text_escapes_csi_osc_and_c1_controls() {
        let text = ExternalText::new("a\u{1b}[31mred\u{7}\u{9b}31mb");

        assert_eq!(text.as_str(), "a\\u{1b}[31mred\\u{7}\\u{9b}31mb");
        assert!(!text.as_str().contains('\u{1b}'));
        assert!(!text.as_str().contains('\u{7}'));
        assert!(!text.as_str().contains('\u{9b}'));
    }

    /// Verifies the upstream text component wraps Unicode by terminal width.
    #[test]
    fn text_block_wraps_wide_unicode_within_width() {
        let text = ExternalText::new("界界界");
        let block = TextBlock::new(&text);
        let lines = block.render(4);

        assert!(lines.iter().all(|line| display_width(line) <= 4));
        assert_eq!(
            lines
                .iter()
                .map(scriberig_tui_kit::ExternalText::as_str)
                .collect::<String>(),
            "界界界"
        );
    }

    /// Verifies Markdown wrapping bounds long unbroken and Unicode content.
    #[test]
    fn markdown_wraps_long_tokens_and_wide_unicode() {
        let lines = super::render_markdown("界界 verylongunbrokentoken", 8);

        assert!(lines.iter().all(|line| display_width(line) <= 8));
    }

    /// Verifies the shared hostile corpus stays inert across kit text adapters.
    #[test]
    fn shared_hostile_corpus_is_safe_for_text_markdown_composer_and_palette() {
        let corpus: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tests/fixtures/terminal_hostile_corpus.json"
        ))
        .expect("hostile corpus JSON");

        for entry in corpus {
            let value = entry["value"].as_str().expect("corpus value");
            let safe = ExternalText::new(value);
            assert!(
                !safe
                    .as_str()
                    .chars()
                    .any(|character| character.is_control() && character != '\n')
            );

            let rendered = render_markdown(value, 24);
            assert!(rendered.iter().all(|line| {
                !line
                    .as_str()
                    .chars()
                    .any(|character| character.is_control() && character != '\n')
            }));

            let composer = Composer::new();
            composer.paste_text(value);
            assert!(
                !composer
                    .text()
                    .as_str()
                    .chars()
                    .any(|character| character.is_control() && character != '\n')
            );

            let palette = StableSelector::new(
                &[SelectionOption {
                    id: "hostile".to_owned(),
                    label: value.to_owned(),
                    description: None,
                }],
                4,
            );
            let lines = palette.render(24);
            assert!(lines.iter().all(|line| {
                !line
                    .as_str()
                    .chars()
                    .any(|character| character.is_control() && character != '\n')
            }));
        }
    }
}

/// Groups tests for editor behavior.
mod composer_tests {
    use super::{Composer, CursorMotion};

    /// Verifies insertion, deletion, and cursor movement use the upstream editor.
    #[test]
    fn edits_and_moves_the_cursor() {
        let composer = Composer::new();
        composer.insert_text("ab");
        composer.move_cursor(CursorMotion::Left);
        composer.insert_text("x");
        composer.delete_backward();

        assert_eq!(composer.text().as_str(), "ab");
    }

    /// Verifies replacing a draft preserves multiline text.
    #[test]
    fn replaces_multiline_text() {
        let composer = Composer::new();
        composer.replace_text("first\nsecond");
        composer.insert_text("!");

        assert_eq!(composer.text().as_str(), "first\nsecond!");
    }

    /// Verifies pasted multiline input remains exact and visibly locates cursor.
    #[test]
    fn pastes_multiline_text_and_renders_a_safe_cursor_marker() {
        let composer = Composer::new();
        composer.paste_text("first\nsecond");

        assert_eq!(composer.text().as_str(), "first\nsecond");
        let rendered = composer
            .render(40)
            .iter()
            .map(scriberig_tui_kit::ExternalText::as_str)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains('▏'));
        assert!(!rendered.contains('\u{1b}'));
    }

    /// Verifies editing preserves Unicode scalar boundaries.
    #[test]
    fn moves_and_edits_around_wide_unicode() {
        let composer = Composer::new();
        composer.insert_text("a界b");
        composer.move_cursor(CursorMotion::Left);
        composer.insert_text("x");
        composer.move_cursor(CursorMotion::Left);
        composer.move_cursor(CursorMotion::DeleteForward);

        assert_eq!(composer.text().as_str(), "a界b");
    }

    /// Verifies control input inserted into a draft remains visible data.
    #[test]
    fn inserted_escape_character_is_visible_text() {
        let composer = Composer::new();
        composer.insert_text("a\u{1b}[2Jb");

        assert_eq!(composer.text().as_str(), "a\\u{1b}[2Jb");
        assert!(
            !composer
                .render(80)
                .iter()
                .any(|line| line.as_str().contains('\u{1b}'))
        );
    }
}

/// Groups tests for Markdown rendering and terminal policy.
mod rendering_tests {
    use super::{ClipboardPolicy, ExternalText, PaletteColor, TerminalPolicy, Tone, style_for};

    /// Verifies headings, paragraphs, and fenced code render as safe text.
    #[test]
    fn markdown_renders_structure_without_terminal_controls() {
        let lines = super::render_markdown("# Heading\n\nbody\n\n```text\ncode\n```", 40);
        let rendered = lines
            .iter()
            .map(ExternalText::as_str)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Heading"));
        assert!(rendered.contains("body"));
        assert!(rendered.contains("code"));
        assert!(!rendered.contains('\u{1b}'));
    }

    /// Verifies Markdown control sequences remain escaped as plain data.
    #[test]
    fn markdown_does_not_pass_through_osc_or_csi_controls() {
        let lines = super::render_markdown("text \u{1b}]52;c;evil\u{7}\u{1b}[2J", 80);
        let rendered = lines
            .iter()
            .map(ExternalText::as_str)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("\\u{1b}]52;c;evil\\u{7}"));
        assert!(rendered.contains("\\u{1b}[2J"));
        assert!(!rendered.contains('\u{1b}'));
    }

    /// Verifies terminal capabilities keep their conservative default policy.
    #[test]
    fn terminal_policy_disables_images_and_requires_clipboard_action() {
        let policy = TerminalPolicy;

        assert!(!policy.images_enabled());
        assert!(!policy.clipboard_allowed(ClipboardPolicy::Disabled, true));
        assert!(!policy.clipboard_allowed(ClipboardPolicy::ExplicitUserAction, false));
        assert!(policy.clipboard_allowed(ClipboardPolicy::ExplicitUserAction, true));
    }

    /// Verifies semantic tones map to bounded style tokens.
    #[test]
    fn tone_mapping_uses_bounded_palette_values() {
        let error = style_for(Tone::Error);

        assert_eq!(error.foreground, PaletteColor::Ansi256(196));
        assert!(error.bold);
        assert_eq!(
            style_for(Tone::BranchDepth(0)).foreground,
            PaletteColor::Ansi256(39)
        );
    }
}

/// Groups tests for stable selection identities.
mod selection_tests {
    use super::{SelectionOption, StableSelector};

    /// Verifies selection returns the caller identity rather than a row index.
    #[test]
    fn selection_keeps_stable_identity() {
        let options = vec![
            SelectionOption {
                id: "entity:alpha".to_string(),
                label: "Alpha".to_string(),
                description: None,
            },
            SelectionOption {
                id: "entity:beta".to_string(),
                label: "Beta".to_string(),
                description: None,
            },
        ];
        let mut selector = StableSelector::new(&options, 5);

        assert_eq!(selector.selected_id(), Some("entity:alpha"));
        selector.move_down();
        assert_eq!(selector.selected_id(), Some("entity:beta"));
    }

    /// Verifies external selection labels are sanitized before upstream render.
    #[test]
    fn selection_label_cannot_emit_terminal_control() {
        let option = SelectionOption {
            id: "stable".to_string(),
            label: "unsafe \u{1b}[2J label".to_string(),
            description: None,
        };
        let selector = StableSelector::new(&[option], 5);
        let lines = selector.render(40);

        assert!(!lines.iter().any(|line| line.as_str().contains('\u{1b}')));
        assert!(
            lines
                .iter()
                .any(|line| line.as_str().contains("\\u{1b}[2J"))
        );
    }
}

/// Keeps an upstream component trait reference scoped to fixture verification.
#[test]
fn markdown_component_is_used_through_a_generic_component_contract() {
    let component = rpi_tui::Markdown::new("plain", 0, 0);

    assert!(component.render(20).join("\n").contains("plain"));
}
