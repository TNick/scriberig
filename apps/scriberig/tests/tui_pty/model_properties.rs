//! Verifies provider-declared model properties through the live upstream TUI.

use super::{TuiPty, test_lock};
use scriberig_session::SessionStore;

/// Exercises the upstream property editor and confirms its value is persisted.
#[test]
fn tui_pty_edits_an_upstream_model_property() {
    let _lock = test_lock();
    let model_config = r#"{
        "providers": {
            "local": {
                "api": "openai-completions",
                "baseUrl": "https://properties.example.test",
                "models": [{
                    "id": "property-model",
                    "name": "Property model",
                    "providerOptionsSchema": {
                        "version": 1,
                        "schema": {
                            "type": "object",
                            "properties": {
                                "region": {
                                    "type": "string",
                                    "enum": ["east", "west"]
                                }
                            },
                            "additionalProperties": false
                        }
                    },
                    "region": "east"
                }]
            }
        }
    }"#;
    let mut tui = TuiPty::start_with_model_config(model_config);
    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    let session = seed_session(&tui);
    tui.send_slow(format!("/switch {session}\r").as_bytes());
    assert!(
        tui.read_until("Property editor conversation"),
        "{}",
        tui.diagnostic()
    );

    tui.send_slow(b"/model\r");
    assert!(tui.read_until("Select model"), "{}", tui.diagnostic());
    tui.send_slow(b"Property model");
    tui.send(b"\r");
    assert!(
        tui.read_until("model property-model"),
        "{}",
        tui.diagnostic()
    );

    tui.send(b"\x1b[15~");
    assert!(
        tui.read_until("Model properties: local/property-model"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.screen().contains("east"), "{}", tui.diagnostic());
    tui.send(b"\r");
    assert!(
        tui.read_until("Model property updated for this process."),
        "{}",
        tui.diagnostic()
    );

    let persisted = std::fs::read(tui.fixture_path().join("models.json"))
        .expect("read persisted upstream model configuration");
    let persisted: serde_json::Value =
        serde_json::from_slice(&persisted).expect("parse persisted model configuration");
    assert_eq!(
        persisted["providers"]["local"]["models"][0]["region"],
        "west"
    );
    assert!(tui.shutdown());
}

/// Seeds an active conversation for model-control operations.
///
/// - `tui`: Running process whose SQLite store receives the session.
///
/// The identity of the durable session.
/// # Returns
/// The new session identity.
fn seed_session(tui: &TuiPty) -> scriberig_core::EntityId {
    let session = scriberig_core::EntityId::new();
    let root = scriberig_session::Branch::new(
        scriberig_core::EntityId::new(),
        "Property editor conversation",
    )
    .expect("create property test branch");
    let store = scriberig_session_sqlite::SqliteSessionStore::open(
        tui.fixture_path().join("scriberig.sqlite"),
    )
    .expect("open property test session store");
    store
        .save(&scriberig_session::Session {
            id: session,
            branches: vec![root],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("seed property test session");
    session
}
