//! Verifies plugin panels use the active upstream overlay manager.

use super::{PluginOverlays, text_panel_options};
use rpi_extensions::ExtensionStatusMailbox;
use rpi_tui::{Component, ProcessTerminal, TUI, TuiAltScreen};

/// Ensures overlay registration, content refresh, and removal use upstream overlays.
#[test]
fn plugin_overlays_follow_mailbox_visibility_and_cleanup() {
    let mailbox = ExtensionStatusMailbox::new();
    let screen = TuiAltScreen::new(Box::new(ProcessTerminal::new()), true, None);
    let minimum_width = screen.terminal().info().columns.saturating_add(1);
    mailbox
        .component(
            9,
            serde_json::json!({
                "op":"register","id":"notice","type":"text",
                "slot":"overlay","anchor":"bottom-left","offsetX":2,
                "offsetY":1,"width":20,"maxHeight":2,"border":false,
                "minScreenWidth":minimum_width
            }),
        )
        .expect("register overlay");
    mailbox
        .component(
            9,
            serde_json::json!({"op":"update","id":"notice","text":"ready"}),
        )
        .expect("set overlay content");
    let mut overlays = PluginOverlays::new();
    assert!(!overlays.synchronize(&screen, Some(&mailbox)));
    assert!(!screen.has_overlay());
    mailbox
        .component(
            9,
            serde_json::json!({
                "op":"configure","id":"notice","minScreenWidth":0
            }),
        )
        .expect("relax overlay minimum width");
    assert!(overlays.synchronize(&screen, Some(&mailbox)));
    assert!(screen.has_overlay());
    let rendered = text_panel_options(
        mailbox
            .panels()
            .into_values()
            .next()
            .expect("visible panel"),
    );
    let component = rpi_tui::TextPanel::new(rendered);
    assert!(
        component
            .render(80)
            .iter()
            .any(|line| line.contains("ready"))
    );
    mailbox
        .component(9, serde_json::json!({"op":"hide","id":"notice"}))
        .expect("hide overlay");
    assert!(overlays.synchronize(&screen, Some(&mailbox)));
    assert!(!screen.has_overlay());
}
