//! Synchronizes plugin-declared overlays with the upstream TUI overlay manager.

use std::collections::BTreeMap;
use std::sync::Arc;

use rpi_extensions::{ExtensionPanel, PanelAnchor, PanelLayout};
use rpi_tui::tui::{OverlayAnchor, OverlayHandle, OverlayOptions, SizeValue, TUI};
use rpi_tui::{Component, TextPanel};

use super::components::text_panel_options;

/// Owns non-capturing upstream overlays registered by isolated plugins.
///
/// # Public members
/// None.
///
/// # Private members
/// - `revision`: Last mailbox revision reflected in the overlay set.
/// - `overlays`: Active upstream overlay handles keyed by mailbox entry.
pub(super) struct PluginOverlays {
    /// Last mailbox revision reflected in the overlay set.
    revision: Option<u64>,
    /// Terminal width used to honor each panel's minimum-width declaration.
    screen_width: Option<usize>,
    /// Active upstream overlay handles keyed by mailbox entry.
    overlays: BTreeMap<String, ActiveOverlay>,
}

/// One declarative panel rendered and positioned by upstream `rpi-tui`.
struct ActiveOverlay {
    /// Panel geometry used to decide when the upstream handle must be recreated.
    geometry: ExtensionPanel,
    /// Text panel whose current contents are rendered by upstream.
    component: Arc<TextPanel>,
    /// Upstream handle used to remove this overlay.
    handle: Arc<dyn OverlayHandle>,
}

impl PluginOverlays {
    /// Creates an empty overlay synchronizer.
    ///
    /// # Returns
    /// A synchronizer ready to reconcile the plugin mailbox.
    pub(super) fn new() -> Self {
        Self {
            revision: None,
            screen_width: None,
            overlays: BTreeMap::new(),
        }
    }

    /// Reconciles visible upstream overlay panels with the plugin mailbox.
    ///
    /// # Parameters
    /// - `screen`: Active upstream terminal runtime.
    /// - `mailbox`: Plugin component registry, when the product has one.
    ///
    /// # Returns
    /// `true` when overlay content or placement changed and a render is needed.
    pub(super) fn synchronize(
        &mut self,
        screen: &dyn TUI,
        mailbox: Option<&rpi_extensions::ExtensionStatusMailbox>,
    ) -> bool {
        let Some(mailbox) = mailbox else {
            return self.clear();
        };
        let revision = mailbox.revision();
        let screen_width = screen.terminal().info().columns;
        if self.revision == Some(revision) && self.screen_width == Some(screen_width) {
            return false;
        }
        self.revision = Some(revision);
        self.screen_width = Some(screen_width);
        let panels: BTreeMap<_, _> = mailbox
            .panels()
            .into_iter()
            .filter(|(_, panel)| {
                panel.layout == PanelLayout::Overlay && screen_width >= panel.min_screen_width
            })
            .collect();
        let mut changed = false;
        self.overlays.retain(|key, active| {
            let Some(panel) = panels.get(key) else {
                active.handle.hide();
                changed = true;
                return false;
            };
            if active.geometry.same_geometry(panel) {
                active
                    .component
                    .set_options(text_panel_options(panel.clone()));
                changed = true;
                return true;
            }
            active.handle.hide();
            changed = true;
            false
        });
        for (key, panel) in panels {
            if self.overlays.contains_key(&key) {
                continue;
            }
            let component = Arc::new(TextPanel::new(text_panel_options(panel.clone())));
            let handle = screen.show_overlay(
                Arc::clone(&component) as Arc<dyn Component>,
                Some(overlay_options(&panel)),
            );
            self.overlays.insert(
                key,
                ActiveOverlay {
                    geometry: panel,
                    component,
                    handle,
                },
            );
            changed = true;
        }
        changed
    }

    /// Removes all upstream plugin overlays when the registry is unavailable.
    ///
    /// # Returns
    /// `true` when at least one overlay was removed.
    fn clear(&mut self) -> bool {
        self.revision = None;
        self.screen_width = None;
        let changed = !self.overlays.is_empty();
        for overlay in self.overlays.values() {
            overlay.handle.hide();
        }
        self.overlays.clear();
        changed
    }
}

/// Converts declarative placement into upstream overlay geometry.
///
/// # Parameters
/// - `panel`: Validated panel declaration.
///
/// # Returns
/// Non-capturing upstream overlay options with its requested anchor and offsets.
fn overlay_options(panel: &ExtensionPanel) -> OverlayOptions {
    OverlayOptions {
        width: Some(SizeValue::Absolute(panel.width)),
        max_height: Some(SizeValue::Absolute(panel.max_height)),
        anchor: match panel.anchor {
            PanelAnchor::TopLeft => OverlayAnchor::TopLeft,
            PanelAnchor::TopCenter => OverlayAnchor::TopCenter,
            PanelAnchor::TopRight => OverlayAnchor::TopRight,
            PanelAnchor::LeftCenter => OverlayAnchor::LeftCenter,
            PanelAnchor::Center => OverlayAnchor::Center,
            PanelAnchor::RightCenter => OverlayAnchor::RightCenter,
            PanelAnchor::BottomLeft => OverlayAnchor::BottomLeft,
            PanelAnchor::BottomCenter => OverlayAnchor::BottomCenter,
            PanelAnchor::BottomRight => OverlayAnchor::BottomRight,
        },
        offset_x: panel.offset_x,
        offset_y: panel.offset_y,
        non_capturing: true,
        ..OverlayOptions::default()
    }
}

#[cfg(test)]
#[path = "plugin_overlays_tests.rs"]
mod tests;
