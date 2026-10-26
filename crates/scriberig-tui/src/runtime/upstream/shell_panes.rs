//! Places the shared semantic panel in the upstream scroll pane its view needs.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rpi_tui::{
    Component, FollowMode, HStack, ScrollView, ScrollViewOptions, StackEntryOptions, VStack,
};

use super::components::{ExtensionSidebarPanels, SidebarSide};
#[path = "shell_panes/layout.rs"]
mod layout;
use super::semantic_views::SemanticViewPanel;
use crate::selection::PrimaryView;
use layout::{stack_composer_options, stack_status_options, stack_viewport_options, status_rows};

/// Upstream scroll pane that displays the shared semantic panel.
///
/// A follow-end scroll view re-pins its viewport to the newest row whenever its content
/// grows. That is the transcript contract for the append-only conversation home, and it
/// is wrong for an auxiliary view such as the candidate review: that document opens as a
/// short loading notice, so follow-end is still engaged, and is then replaced by a
/// candidate diff taller than the terminal, which pushes the change, base, result, and
/// validation status lines above the visible rows. Keeping the two contracts in separate
/// panes preserves each one without local clipping or truncation.
///
/// # Public members
/// - `new`: Builds the panes and arranges the one matching the initial view.
/// - `root`: Layout root handed to the upstream terminal.
/// - `synchronize`: Re-arranges and re-anchors the panes for the active view.
/// - `transcript`: Test-only accessor for the conversation pane.
/// - `document`: Test-only accessor for the auxiliary document pane.
///
/// # Private members
/// - `status`: Fixed status and key-hint pane.
/// - `composer`: Fixed composer pane.
/// - `transcript`: Follow-end transcript scroll view over the semantic panel.
/// - `document`: Non-following document scroll view over the same semantic panel.
/// - `stack`: Shell stack rebuilt when the active pane changes.
/// - `anchored_view`: View used by the previous anchor.
/// - `active_pane`: Pane currently placed between the fixed panes.
/// - `extension_mailbox`: Optional plugin component registry used for footer sizing.
/// - `extension_revision`: Last mailbox revision reflected in the shell layout.
/// - `status_rows`: Fixed status area height, including visible plugin footer rows.
/// - `left_sidebar`: Dynamic plugin panels anchored to the left edge.
/// - `right_sidebar`: Dynamic plugin panels anchored to the right edge.
/// - `viewport_width`: Whole-terminal width shared with the sidebars.
/// - `sized_width`: Width reflected by the last shell arrangement.
pub(super) struct ShellPanes {
    /// Fixed status and key-hint pane.
    status: Arc<dyn Component>,
    /// Fixed composer pane.
    composer: Arc<dyn Component>,
    /// Follow-end transcript scroll view over the semantic panel.
    transcript: Arc<ScrollView>,
    /// Non-following document scroll view over the same semantic panel.
    document: Arc<ScrollView>,
    /// Shell stack rebuilt when the active pane changes.
    stack: Arc<VStack>,
    /// View used by the previous anchor.
    anchored_view: Option<PrimaryView>,
    /// Pane currently placed between the fixed panes.
    active_pane: ScrollPane,
    /// Optional plugin component registry used for footer sizing.
    extension_mailbox: Option<rpi_extensions::ExtensionStatusMailbox>,
    /// Last mailbox revision reflected in the shell layout.
    extension_revision: Option<u64>,
    /// Fixed status area height, including visible plugin footer rows.
    status_rows: usize,
    /// Dynamic plugin panels anchored to the left edge.
    left_sidebar: Option<Arc<ExtensionSidebarPanels>>,
    /// Dynamic plugin panels anchored to the right edge.
    right_sidebar: Option<Arc<ExtensionSidebarPanels>>,
    /// Whole-terminal width shared with the sidebars.
    viewport_width: Arc<AtomicUsize>,
    /// Width reflected by the last shell arrangement.
    sized_width: Option<usize>,
}

/// Which of the two scroll panes the active view occupies.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScrollPane {
    /// Follow-end pane for the append-only conversation home.
    Transcript,
    /// Pane that keeps the reader's position in a document replaced as a whole.
    Document,
}

impl ScrollPane {
    /// Selects the pane whose scrolling contract matches one semantic view.
    ///
    /// - `view`: Active semantic view.
    ///
    /// # Returns
    /// The transcript pane for the conversation home, and the document pane for every
    /// view rebuilt from a read model, including the candidate review.
    fn for_view(view: PrimaryView) -> Self {
        if view == PrimaryView::Conversation {
            Self::Transcript
        } else {
            Self::Document
        }
    }
}

impl ShellPanes {
    /// Creates the shell panes and arranges the pane that matches the initial view.
    ///
    /// - `panel`: Shared semantic panel rendered by both scroll panes.
    /// - `status`: Fixed status and key-hint pane.
    /// - `composer`: Fixed composer pane.
    /// - `initial_view`: View the shell opens with.
    /// - `extension_mailbox`: Optional registry of host-mediated UI panels.
    ///
    /// # Returns
    /// The panes with the status pane, the matching scroll pane, and the composer.
    pub(super) fn new(
        panel: &Arc<SemanticViewPanel>,
        status: Arc<dyn Component>,
        composer: Arc<dyn Component>,
        initial_view: PrimaryView,
        extension_mailbox: Option<rpi_extensions::ExtensionStatusMailbox>,
    ) -> Self {
        let viewport_width = Arc::new(AtomicUsize::new(80));
        let status_rows = status_rows(extension_mailbox.as_ref(), 80);
        let extension_revision = extension_mailbox
            .as_ref()
            .map(rpi_extensions::ExtensionStatusMailbox::revision);
        let left_sidebar = extension_mailbox.as_ref().map(|mailbox| {
            Arc::new(ExtensionSidebarPanels::new(
                mailbox.clone(),
                SidebarSide::Left,
                Arc::clone(&viewport_width),
            ))
        });
        let right_sidebar = extension_mailbox.as_ref().map(|mailbox| {
            Arc::new(ExtensionSidebarPanels::new(
                mailbox.clone(),
                SidebarSide::Right,
                Arc::clone(&viewport_width),
            ))
        });
        let shell = Self {
            status,
            composer,
            transcript: Arc::new(ScrollView::primary(Arc::clone(panel) as Arc<dyn Component>)),
            document: Arc::new(ScrollView::new(
                Arc::clone(panel) as Arc<dyn Component>,
                ScrollViewOptions {
                    // A replaced document must stay where the reader left it, and its
                    // first rows must stay visible when it grows past the viewport.
                    follow: FollowMode::None,
                    primary: true,
                    ..ScrollViewOptions::default()
                },
            )),
            stack: Arc::new(VStack::new()),
            anchored_view: Some(initial_view),
            active_pane: ScrollPane::for_view(initial_view),
            extension_mailbox,
            extension_revision,
            status_rows,
            left_sidebar,
            right_sidebar,
            viewport_width,
            sized_width: None,
        };
        shell.arrange();
        shell
    }

    /// Returns the shell layout root for the upstream terminal.
    ///
    /// # Returns
    /// The stack holding the status pane, the active scroll pane, and the composer.
    pub(super) fn root(&self) -> Arc<dyn Component> {
        Arc::clone(&self.stack) as Arc<dyn Component>
    }

    /// Returns the follow-end transcript pane for a viewport test.
    ///
    /// # Returns
    /// The scroll pane the conversation home occupies.
    #[cfg(test)]
    pub(super) fn transcript(&self) -> &Arc<ScrollView> {
        &self.transcript
    }

    /// Returns the document pane for a viewport test.
    ///
    /// # Returns
    /// The scroll pane the auxiliary semantic views occupy.
    #[cfg(test)]
    pub(super) fn document(&self) -> &Arc<ScrollView> {
        &self.document
    }

    /// Returns the plugin component registry for upstream overlay composition.
    ///
    /// # Returns
    /// The active mailbox, or `None` for a product session without plugins.
    pub(super) fn extension_mailbox(&self) -> Option<&rpi_extensions::ExtensionStatusMailbox> {
        self.extension_mailbox.as_ref()
    }

    /// Re-anchors the shell for the active view and swaps the pane when needed.
    ///
    /// The conversation home resumes following its newest row, and an auxiliary view
    /// resumes at its first row. A view that did not change keeps the reader's manual
    /// scroll position in the pane it already occupies.
    ///
    /// - `view`: Active semantic view.
    ///
    /// # Returns
    /// `true` when the shell stack was rebuilt, so the caller must repaint.
    pub(super) fn synchronize(&mut self, view: PrimaryView) -> bool {
        if self.anchored_view == Some(view) {
            return false;
        }
        self.anchored_view = Some(view);
        let pane = ScrollPane::for_view(view);
        let rearranged = pane != self.active_pane;
        if rearranged {
            self.active_pane = pane;
            self.arrange();
        }
        match pane {
            ScrollPane::Transcript => self.transcript.scroll_to_end(),
            ScrollPane::Document => self.document.scroll_to_start(),
        }
        rearranged
    }

    /// Rebuilds the fixed status area when plugin footer registrations change.
    ///
    /// # Returns
    /// `true` when the shell height allocation changed and requires repainting.
    /// - `screen_width`: Current terminal width used for panel layout.
    pub(super) fn synchronize_extension_components(&mut self, screen_width: usize) -> bool {
        self.viewport_width.store(screen_width, Ordering::Relaxed);
        let revision = self
            .extension_mailbox
            .as_ref()
            .map(rpi_extensions::ExtensionStatusMailbox::revision);
        if self.extension_revision == revision && self.sized_width == Some(screen_width) {
            return false;
        }
        self.extension_revision = revision;
        self.sized_width = Some(screen_width);
        self.status_rows = status_rows(self.extension_mailbox.as_ref(), screen_width);
        self.arrange();
        true
    }

    /// Rebuilds the stack so only the active scroll pane takes the leftover rows.
    ///
    /// The scroll pane must receive only the terminal rows left over by the fixed panes.
    /// An auto-basis stack entry measures a scroll view at its full content height, which
    /// pushes the composer off the terminal and leaves the viewport as tall as the
    /// content, so the visible document stays pinned to its oldest rows.
    fn arrange(&self) {
        self.stack.clear();
        self.stack.add_child_with_options(
            Arc::clone(&self.status),
            stack_status_options(self.status_rows),
        );
        let active: Arc<dyn Component> = match self.active_pane {
            ScrollPane::Transcript => Arc::clone(&self.transcript) as Arc<dyn Component>,
            ScrollPane::Document => Arc::clone(&self.document) as Arc<dyn Component>,
        };
        let viewport = Arc::new(HStack::new());
        if let Some(sidebar) = self
            .left_sidebar
            .as_ref()
            .filter(|sidebar| sidebar.width() > 0)
        {
            viewport.add_child_with_options(
                Arc::clone(sidebar) as Arc<dyn Component>,
                StackEntryOptions::with_basis(sidebar.width()).min_size(1),
            );
        }
        viewport
            .add_child_with_options(active, StackEntryOptions::with_basis(0).grow(1).min_size(1));
        if let Some(sidebar) = self
            .right_sidebar
            .as_ref()
            .filter(|sidebar| sidebar.width() > 0)
        {
            viewport.add_child_with_options(
                Arc::clone(sidebar) as Arc<dyn Component>,
                StackEntryOptions::with_basis(sidebar.width()).min_size(1),
            );
        }
        self.stack
            .add_child_with_options(viewport as Arc<dyn Component>, stack_viewport_options());
        self.stack
            .add_child_with_options(Arc::clone(&self.composer), stack_composer_options());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::mpsc;

    use rpi_extensions::ExtensionStatusMailbox;
    use rpi_tui::{Keybindings, Text};

    use super::ShellPanes;
    use crate::runtime::upstream::semantic_views::SemanticViewPanel;
    use crate::selection::PrimaryView;
    use crate::state::TuiState;

    /// Confirms the shell places plugin sidebars around its semantic pane and responds to resize.
    #[test]
    fn shell_composes_sidebars_in_the_upstream_horizontal_layout() {
        let mailbox = ExtensionStatusMailbox::new();
        mailbox
            .component(
                4,
                serde_json::json!({
                    "op":"register","id":"left","type":"text",
                    "slot":"sidebar.left","width":20,"maxHeight":2,
                    "minScreenWidth":90,"border":false
                }),
            )
            .expect("register sidebar");
        mailbox
            .component(
                4,
                serde_json::json!({"op":"update","id":"left","text":"side status"}),
            )
            .expect("update sidebar");
        let state = Arc::new(Mutex::new(TuiState::default()));
        let (sender, _receiver) = mpsc::channel();
        let semantic_panel = Arc::new(SemanticViewPanel::new(
            state,
            sender,
            Arc::new(Keybindings::new()),
        ));
        let mut panes = ShellPanes::new(
            &semantic_panel,
            Arc::new(Text::new("status", 0, 0)),
            Arc::new(Text::new("composer", 0, 0)),
            PrimaryView::Conversation,
            Some(mailbox),
        );
        assert!(
            !panes
                .root()
                .render(100)
                .iter()
                .any(|row| row.contains("side status"))
        );
        assert!(panes.synchronize_extension_components(100));
        assert!(
            panes
                .root()
                .render(100)
                .iter()
                .any(|row| row.contains("side status"))
        );
    }
}
