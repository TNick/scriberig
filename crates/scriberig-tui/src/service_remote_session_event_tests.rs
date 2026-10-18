//! Verifies remote session refresh events cross the TUI adapter boundary.

use super::super::super::service_remote_mapping::map_remote_event;
use scriberig_core::EntityId;
use scriberig_protocol::{EntityReference, Event, EventKind, OperationReference};

/// Preserves remote session refresh events across the TUI adapter boundary.
#[test]
fn remote_session_change_event_is_preserved() {
    let session = EntityId::from_bytes([8; 16]);
    let event = Event {
        sequence: 4,
        kind: EventKind::SessionChanged,
        entity: Some(EntityReference {
            id: session.to_string(),
        }),
        revision: None,
        snapshot: None,
        operation: None,
        activity_kind: None,
        activity_state: None,
        metadata: Vec::new(),
    };

    assert!(matches!(
        map_remote_event(&event),
        Some(scriberig_app::ApplicationEvent::SessionChanged { session: value })
            if value == session
    ));
}

/// Preserves safe model/thinking selection activity from the server event stream.
#[test]
fn remote_session_selection_event_is_preserved() {
    let session = EntityId::from_bytes([9; 16]);
    let event = Event {
        sequence: 6,
        kind: EventKind::SessionSelectionChanged,
        entity: Some(EntityReference {
            id: session.to_string(),
        }),
        revision: None,
        snapshot: None,
        operation: None,
        activity_kind: Some(scriberig_protocol::SessionActivityKind::ModelSelectionChanged),
        activity_state: None,
        metadata: vec![("summary".to_owned(), "Model selected: Model A".to_owned())],
    };

    assert!(matches!(
        map_remote_event(&event),
        Some(scriberig_app::ApplicationEvent::SessionSelectionChanged {
            session: value,
            activity_kind,
            summary,
        }) if value == session
            && activity_kind == scriberig_session::AgentActivityKind::ModelSelectionChanged
            && summary == "Model selected: Model A"
    ));
}

/// Preserves transient remote agent activity across the TUI adapter boundary.
#[test]
fn remote_agent_activity_event_is_preserved() {
    let event = Event {
        sequence: 5,
        kind: EventKind::AgentActivity,
        entity: None,
        revision: None,
        snapshot: None,
        operation: Some(OperationReference {
            id: "op_agent-1".to_owned(),
        }),
        activity_kind: Some(scriberig_protocol::SessionActivityKind::ToolInvocation),
        activity_state: Some(scriberig_protocol::SessionActivityState::Running),
        metadata: vec![
            ("branch".to_owned(), "branch-1".to_owned()),
            ("summary".to_owned(), "Tool: search".to_owned()),
        ],
    };

    assert!(matches!(
        map_remote_event(&event),
        Some(scriberig_app::ApplicationEvent::AgentActivity {
            branch,
            activity_kind,
            state,
            summary,
            ..
        }) if branch == "branch-1"
            && activity_kind == scriberig_session::AgentActivityKind::ToolInvocation
            && state == scriberig_session::AgentActivityState::Running
            && summary == "Tool: search"
    ));
}
