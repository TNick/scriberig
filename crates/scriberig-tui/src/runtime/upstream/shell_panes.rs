//! Places the shared semantic panel in the upstream scroll pane its view needs.

use std::sync::Arc;

use rpi_tui::{Component, FollowMode, ScrollView, ScrollViewOptions, StackEntryOptions, VStack};

use super::semantic_views::SemanticViewPanel;
use crate::selection::PrimaryView;

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
    ///
    /// # Returns
    /// The panes with the status pane, the matching scroll pane, and the composer.
    pub(super) fn new(
        panel: &Arc<SemanticViewPanel>,
        status: Arc<dyn Component>,
        composer: Arc<dyn Component>,
        initial_view: PrimaryView,
    ) -> Self {
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

    /// Rebuilds the stack so only the active scroll pane takes the leftover rows.
    ///
    /// The scroll pane must receive only the terminal rows left over by the fixed panes.
    /// An auto-basis stack entry measures a scroll view at its full content height, which
    /// pushes the composer off the terminal and leaves the viewport as tall as the
    /// content, so the visible document stays pinned to its oldest rows.
    fn arrange(&self) {
        self.stack.clear();
        self.stack
            .add_child_with_options(Arc::clone(&self.status), stack_status_options());
        let active: Arc<dyn Component> = match self.active_pane {
            ScrollPane::Transcript => Arc::clone(&self.transcript) as Arc<dyn Component>,
            ScrollPane::Document => Arc::clone(&self.document) as Arc<dyn Component>,
        };
        self.stack
            .add_child_with_options(active, stack_viewport_options());
        self.stack
            .add_child_with_options(Arc::clone(&self.composer), stack_composer_options());
    }
}

/// Layout options for the fixed status and key-hint pane.
///
/// # Returns
/// A fixed three-row entry that never shrinks below two rows.
fn stack_status_options() -> StackEntryOptions {
    StackEntryOptions::with_basis(3).min_size(2)
}

/// Layout options for the active scroll pane.
///
/// # Returns
/// A growable entry that claims exactly the rows the fixed panes leave.
fn stack_viewport_options() -> StackEntryOptions {
    StackEntryOptions::with_basis(0)
        .grow(1)
        .shrink(1)
        .min_size(1)
}

/// Layout options for the fixed composer pane.
///
/// # Returns
/// A fixed five-row entry that never shrinks below three rows.
fn stack_composer_options() -> StackEntryOptions {
    StackEntryOptions::with_basis(5).min_size(3)
}
