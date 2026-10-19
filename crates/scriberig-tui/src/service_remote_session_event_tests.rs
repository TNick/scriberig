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

/// Preserves typed durable plan changes across the remote TUI adapter.
#[test]
fn remote_plan_activity_event_is_preserved() {
    let session = EntityId::from_bytes([6; 16]);
    let plan = EntityId::from_bytes([7; 16]);
    let node = EntityId::from_bytes([8; 16]);
    let event = Event {
        sequence: 7,
        kind: EventKind::PlanActivity,
        entity: Some(EntityReference {
            id: plan.to_string(),
        }),
        revision: None,
        snapshot: None,
        operation: None,
        activity_kind: Some(scriberig_protocol::SessionActivityKind::PlanNodeReady),
        activity_state: Some(scriberig_protocol::SessionActivityState::Running),
        metadata: vec![
            ("session".to_owned(), session.to_string()),
            ("node".to_owned(), node.to_string()),
            ("summary".to_owned(), "Node ready".to_owned()),
        ],
    };

    assert!(matches!(
        map_remote_event(&event),
        Some(scriberig_app::ApplicationEvent::PlanActivity {
            session: actual_session,
            plan: actual_plan,
            node: Some(actual_node),
            activity_kind: scriberig_session::SessionActivityKind::PlanNodeReady,
            state: scriberig_session::SessionActivityState::Running,
            summary,
        }) if actual_session == session
            && actual_plan == plan
            && actual_node == node
            && summary == "Node ready"
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
