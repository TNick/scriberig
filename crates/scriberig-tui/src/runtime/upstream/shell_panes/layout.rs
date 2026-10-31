//! Computes upstream shell layout options and bounded status height.

use rpi_tui::StackEntryOptions;

/// Layout options for the fixed status and key-hint pane.
///
/// # Parameters
/// - `rows`: Number of fixed status rows to allocate.
///
/// # Returns
/// A fixed entry that never shrinks below two rows.
pub(super) fn stack_status_options(rows: usize) -> StackEntryOptions {
    StackEntryOptions::with_basis(rows).min_size(2)
}

/// Computes the bounded fixed height required by the status and plugin footer rows.
///
/// # Parameters
/// - `mailbox`: Optional registry containing visible plugin panels.
/// - `screen_width`: Current terminal width used to filter panel visibility.
///
/// # Returns
/// The status height, capped to preserve space for the semantic viewport.
pub(super) fn status_rows(
    mailbox: Option<&rpi_extensions::ExtensionStatusMailbox>,
    screen_width: usize,
) -> usize {
    let plugin_rows = mailbox.map_or(0, |mailbox| {
        mailbox
            .panels()
            .into_values()
            .filter(|panel| {
                panel.layout == rpi_extensions::PanelLayout::Footer
                    && screen_width >= panel.min_screen_width
            })
            .map(|panel| panel.max_height.min(80))
            .sum::<usize>()
            .min(12)
    });
    3 + plugin_rows
}

/// Layout options for the active scroll pane.
///
/// # Returns
/// A growable entry that claims exactly the rows the fixed panes leave.
pub(super) fn stack_viewport_options() -> StackEntryOptions {
    StackEntryOptions::with_basis(0)
        .grow(1)
        .shrink(1)
        .min_size(1)
}

/// Layout options for the fixed composer pane.
///
/// # Returns
/// A fixed five-row entry that never shrinks below three rows.
pub(super) fn stack_composer_options() -> StackEntryOptions {
    StackEntryOptions::with_basis(5).min_size(3)
}
