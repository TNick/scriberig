//! Adapts upstream plugin text panels to the upstream TUI text renderer.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rpi_extensions::{
    ExtensionPanel, ExtensionStatusMailbox, PanelAnchor, PanelHighlightColor, PanelLayout,
};
use rpi_tui::{
    Component, TextPanel, TextPanelHighlight, TextPanelHighlightColor, TextPanelOptions,
};

/// Identifies which horizontal edge receives plugin sidebar panels.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SidebarSide {
    /// Left edge of the semantic viewport.
    Left,
    /// Right edge of the semantic viewport.
    Right,
}

/// Renders registered footer panels using the upstream bounded text component.
///
/// # Public members
/// None.
///
/// # Private members
/// - `mailbox`: Shared upstream extension component registry.
pub struct ExtensionFooterPanels {
    /// Shared upstream extension component registry.
    mailbox: ExtensionStatusMailbox,
}

impl ExtensionFooterPanels {
    /// Creates a footer panel adapter over the active plugin mailbox.
    ///
    /// # Parameters
    /// - `mailbox`: Upstream registry receiving isolated plugin updates.
    ///
    /// # Returns
    /// A passive renderer for visible `footer.after` components.
    pub(crate) fn new(mailbox: ExtensionStatusMailbox) -> Self {
        Self { mailbox }
    }
}

impl Component for ExtensionFooterPanels {
    /// Renders visible footer panels through `rpi_tui::TextPanel`.
    ///
    /// # Parameters
    /// - `width`: Available terminal columns.
    ///
    /// # Returns
    /// Bounded rows from the visible footer panels.
    fn render(&self, width: usize) -> Vec<String> {
        let mut rows = Vec::new();
        for panel in
            self.mailbox.panels().into_values().filter(|panel| {
                panel.layout == PanelLayout::Footer && width >= panel.min_screen_width
            })
        {
            let rendered = TextPanel::new(text_panel_options(panel)).render(width);
            rows.extend(rendered);
            if rows.len() >= 12 {
                rows.truncate(12);
                break;
            }
        }
        rows
    }

    /// Keeps the passive footer component outside keyboard focus.
    fn invalidate(&self) {}

    /// Provides upstream component introspection.
    ///
    /// # Returns
    /// A type-erased reference to this component.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Renders plugin sidebar panels for one edge of the semantic viewport.
///
/// # Public members
/// None.
///
/// # Private members
/// - `mailbox`: Shared upstream plugin component registry.
/// - `side`: Horizontal edge selected for this panel group.
/// - `screen_width`: Current whole-terminal width for minimum-width rules.
pub struct ExtensionSidebarPanels {
    /// Shared upstream plugin component registry.
    mailbox: ExtensionStatusMailbox,
    /// Horizontal edge selected for this panel group.
    side: SidebarSide,
    /// Current whole-terminal width for minimum-width rules.
    screen_width: Arc<AtomicUsize>,
}

impl ExtensionSidebarPanels {
    /// Creates a dynamic sidebar adapter over the plugin component registry.
    ///
    /// # Parameters
    /// - `mailbox`: Upstream registry receiving isolated plugin updates.
    /// - `side`: Horizontal edge this adapter renders.
    /// - `screen_width`: Shared whole-terminal width updated by the runtime.
    ///
    /// # Returns
    /// A passive renderer for visible sidebar panels at that edge.
    pub fn new(
        mailbox: ExtensionStatusMailbox,
        side: SidebarSide,
        screen_width: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            mailbox,
            side,
            screen_width,
        }
    }

    /// Returns the largest declared panel width for this edge.
    ///
    /// # Returns
    /// Zero when no visible panel is registered on the requested edge.
    pub fn width(&self) -> usize {
        self.mailbox
            .panels()
            .into_values()
            .filter(|panel| {
                panel.layout == PanelLayout::Sidebar
                    && panel_on_side(panel, self.side)
                    && self.screen_width.load(Ordering::Relaxed) >= panel.min_screen_width
            })
            .map(|panel| panel.width)
            .max()
            .unwrap_or_default()
    }
}

impl Component for ExtensionSidebarPanels {
    /// Renders only panels assigned to this sidebar edge.
    ///
    /// # Parameters
    /// - `width`: Available terminal columns.
    ///
    /// # Returns
    /// Bounded rows from the visible sidebar panels.
    fn render(&self, width: usize) -> Vec<String> {
        let mut rows = Vec::new();
        for panel in self.mailbox.panels().into_values().filter(|panel| {
            panel.layout == PanelLayout::Sidebar
                && panel_on_side(panel, self.side)
                && self.screen_width.load(Ordering::Relaxed) >= panel.min_screen_width
        }) {
            rows.extend(TextPanel::new(text_panel_options(panel)).render(width));
            if rows.len() >= 80 {
                rows.truncate(80);
                break;
            }
        }
        rows
    }

    /// Keeps passive plugin sidebar content outside keyboard focus.
    fn invalidate(&self) {}

    /// Provides upstream component introspection.
    ///
    /// # Returns
    /// A type-erased reference to this component.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Converts a mailbox panel into bounded upstream text-panel options.
///
/// # Parameters
/// - `panel`: Validated declarative panel from the upstream mailbox.
///
/// # Returns
/// Renderer options that preserve safe text and highlight metadata.
pub fn text_panel_options(panel: ExtensionPanel) -> TextPanelOptions {
    TextPanelOptions {
        width: panel.width,
        max_height: panel.max_height,
        border: panel.border,
        title: panel.title,
        lines: panel.lines,
        highlights: panel
            .highlights
            .into_iter()
            .map(|highlight| TextPanelHighlight {
                text: highlight.text,
                color: match highlight.color {
                    PanelHighlightColor::Warning => TextPanelHighlightColor::Warning,
                    PanelHighlightColor::Error => TextPanelHighlightColor::Error,
                },
            })
            .collect(),
    }
}

/// Resolves a panel anchor to its sidebar edge.
///
/// # Parameters
/// - `panel`: Validated declarative panel.
/// - `side`: Requested horizontal edge.
///
/// # Returns
/// `true` when the panel's anchor belongs to that edge.
fn panel_on_side(panel: &ExtensionPanel, side: SidebarSide) -> bool {
    match side {
        SidebarSide::Left => matches!(
            panel.anchor,
            PanelAnchor::TopLeft | PanelAnchor::LeftCenter | PanelAnchor::BottomLeft
        ),
        SidebarSide::Right => matches!(
            panel.anchor,
            PanelAnchor::TopRight
                | PanelAnchor::RightCenter
                | PanelAnchor::BottomRight
                | PanelAnchor::TopCenter
                | PanelAnchor::Center
                | PanelAnchor::BottomCenter
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{ExtensionFooterPanels, ExtensionSidebarPanels, SidebarSide};
    use rpi_extensions::ExtensionStatusMailbox;
    use rpi_tui::Component;

    /// Confirms footer updates flow through the bounded upstream text renderer.
    #[test]
    fn footer_component_renders_bounded_text_and_respects_minimum_width() {
        let mailbox = ExtensionStatusMailbox::new();
        mailbox
            .component(
                7,
                serde_json::json!({
                    "op":"register","id":"status","type":"text",
                    "slot":"footer.after","width":12,"maxHeight":1,
                    "border":false,"minScreenWidth":40
                }),
            )
            .expect("register footer panel");
        mailbox
            .component(
                7,
                serde_json::json!({
                    "op":"update","id":"status","text":"abcdefghijklmno"
                }),
            )
            .expect("update footer panel");

        let component = ExtensionFooterPanels::new(mailbox.clone());
        let rows = component.render(40);
        assert_eq!(rows.len(), 1);
        assert!(rpi_tui::visible_width(&rows[0]) <= 12);
        assert_eq!(component.render(39).len(), 0);
    }

    /// Confirms left and right plugin sidebars stay in their upstream layout slots.
    #[test]
    fn sidebar_components_render_only_their_declared_edge() {
        let mailbox = ExtensionStatusMailbox::new();
        for (id, slot, text) in [
            ("left", "sidebar.left", "left panel"),
            ("right", "sidebar.right", "right panel"),
        ] {
            mailbox
                .component(
                    3,
                    serde_json::json!({"op":"register","id":id,"type":"text","slot":slot,"width":20,"maxHeight":2,"minScreenWidth":50,"border":false}),
                )
                .expect("register sidebar");
            mailbox
                .component(3, serde_json::json!({"op":"update","id":id,"text":text}))
                .expect("update sidebar");
        }
        let screen_width = Arc::new(AtomicUsize::new(80));
        let left = ExtensionSidebarPanels::new(
            mailbox.clone(),
            SidebarSide::Left,
            Arc::clone(&screen_width),
        );
        let right =
            ExtensionSidebarPanels::new(mailbox, SidebarSide::Right, Arc::clone(&screen_width));
        assert_eq!(left.width(), 20);
        assert_eq!(right.width(), 20);
        assert!(left.render(20).iter().any(|row| row.contains("left panel")));
        assert!(
            !left
                .render(20)
                .iter()
                .any(|row| row.contains("right panel"))
        );
        assert!(
            right
                .render(20)
                .iter()
                .any(|row| row.contains("right panel"))
        );
        assert!(
            !right
                .render(20)
                .iter()
                .any(|row| row.contains("left panel"))
        );
        screen_width.store(49, Ordering::Relaxed);
        assert_eq!(left.width(), 0);
        assert_eq!(left.render(20).len(), 0);
    }
}
