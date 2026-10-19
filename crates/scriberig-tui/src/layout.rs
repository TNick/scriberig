//! Computes responsive shell regions without performing rendering or I/O.

use ratatui::layout::Rect;

/// Responsive body layout selected from the terminal width.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutClass {
    /// Explorer, primary, and inspector are visible.
    Wide,
    /// Explorer and primary are visible.
    Medium,
    /// Only the primary view is visible.
    Narrow,
    /// The terminal is too small for the shell and gets a clear message.
    TooSmall,
}

/// All shell regions for one frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellLayout {
    /// The selected responsive width class.
    pub class: LayoutClass,
    /// The context bar rectangle.
    pub context: Rect,
    /// The explorer rectangle, when visible.
    pub explorer: Option<Rect>,
    /// The primary rectangle.
    pub primary: Rect,
    /// The inspector rectangle, when visible.
    pub inspector: Option<Rect>,
    /// The status rectangle.
    pub status: Rect,
}

/// Computes one single-column product shell for a terminal rectangle.
#[must_use]
/// - `area`: Argument named `area`.
///
/// # Returns
/// The operation result.
pub fn shell_layout(area: Rect) -> ShellLayout {
    let vertical = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Length(2),
        ratatui::layout::Constraint::Min(1),
        ratatui::layout::Constraint::Length(1),
    ])
    .split(area);
    let class = match area.width {
        120.. => LayoutClass::Wide,
        80..=119 => LayoutClass::Medium,
        20..=79 => LayoutClass::Narrow,
        _ => LayoutClass::TooSmall,
    };
    let body = vertical[1];
    ShellLayout {
        class,
        context: vertical[0],
        explorer: None,
        primary: body,
        inspector: None,
        status: vertical[2],
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{LayoutClass, shell_layout};

    /// Verifies the three documented responsive width classes.
    #[test]
    fn selects_required_width_classes() {
        for (width, class) in [
            (140, LayoutClass::Wide),
            (100, LayoutClass::Medium),
            (72, LayoutClass::Narrow),
        ] {
            let layout = shell_layout(Rect::new(0, 0, width, 30));
            assert_eq!(layout.class, class);
            assert!(layout.explorer.is_none());
            assert!(layout.inspector.is_none());
            assert_eq!(layout.primary.width, width);
        }
    }

    /// Verifies very small dimensions remain bounded and safe.
    #[test]
    fn handles_small_terminal() {
        let layout = shell_layout(Rect::new(0, 0, 10, 3));
        assert_eq!(layout.class, LayoutClass::TooSmall);
    }
}
