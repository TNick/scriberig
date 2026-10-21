//! Converts untrusted Markdown into safe plain terminal text.

use rpi_tui::Component;

use crate::text::ExternalText;

/// Renders external Markdown through the upstream parser and strips styling.
///
/// # Parameters
/// - `markdown`: Untrusted Markdown content.
/// - `width`: Maximum rendering width in terminal columns.
///
/// # Returns
/// Safe plain lines with Markdown structure rendered as text.
#[must_use]
pub fn render_markdown(markdown: &str, width: usize) -> Vec<ExternalText> {
    let safe_markdown = ExternalText::new(markdown);
    let component = rpi_tui::Markdown::new(safe_markdown.as_str(), 0, 0);
    component
        .render(width)
        .iter()
        .map(|line| ExternalText::new(&rpi_tui::strip_ansi(line)))
        .collect()
}
