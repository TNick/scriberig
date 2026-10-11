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
    /// The key-hint rectangle.
    pub hints: Rect,
}

/// Computes shell regions for a terminal rectangle.
#[must_use]
/// - `area`: Argument named `area`.
///
/// # Returns
/// The value produced by the declaration.
pub fn shell_layout(area: Rect) -> ShellLayout {
    let vertical = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Length(1),
        ratatui::layout::Constraint::Min(1),
        ratatui::layout::Constraint::Length(1),
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
    let (explorer, primary, inspector) = match class {
        LayoutClass::Wide => {
            let columns = ratatui::layout::Layout::horizontal([
                ratatui::layout::Constraint::Percentage(25),
                ratatui::layout::Constraint::Percentage(50),
                ratatui::layout::Constraint::Percentage(25),
            ])
            .split(body);
            (Some(columns[0]), columns[1], Some(columns[2]))
        }
        LayoutClass::Medium => {
            let columns = ratatui::layout::Layout::horizontal([
                ratatui::layout::Constraint::Percentage(30),
                ratatui::layout::Constraint::Percentage(70),
            ])
            .split(body);
            (Some(columns[0]), columns[1], None)
        }
        LayoutClass::Narrow | LayoutClass::TooSmall => (None, body, None),
    };
    ShellLayout {
        class,
        context: vertical[0],
        explorer,
        primary,
        inspector,
        status: vertical[2],
        hints: vertical[3],
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{LayoutClass, shell_layout};

    /// Verifies the three documented responsive width classes.
    #[test]
    fn selects_required_width_classes() {
        assert_eq!(
            shell_layout(Rect::new(0, 0, 140, 40)).class,
            LayoutClass::Wide
        );
        assert_eq!(
            shell_layout(Rect::new(0, 0, 100, 30)).class,
            LayoutClass::Medium
        );
        assert_eq!(
            shell_layout(Rect::new(0, 0, 72, 24)).class,
            LayoutClass::Narrow
        );
    }

    /// Verifies very small dimensions remain bounded and safe.
    #[test]
    fn handles_small_terminal() {
        let layout = shell_layout(Rect::new(0, 0, 10, 3));
        assert_eq!(layout.class, LayoutClass::TooSmall);
    }
}
