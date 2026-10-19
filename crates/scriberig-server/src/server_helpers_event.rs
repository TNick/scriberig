//! Converts application lifecycle events into protocol envelopes.

use crate::server::{ApplicationEvent, Event, EventKind};

/// Converts one application lifecycle event to a stable protocol envelope.
///
/// - `event`: Transport-independent application event.
///
/// A protocol event with safe metadata and stable references.
#[allow(clippy::too_many_lines)]
/// # Returns
/// The operation result.
pub fn event_from_application(event: ApplicationEvent) -> Event {
    let mut entity = None;
    let mut revision = None;
    let mut snapshot = None;
    let mut operation = None;
    let mut typed_activity_kind = None;
    let mut typed_activity_state = None;
    let mut metadata = Vec::new();
    let kind = match event {
        ApplicationEvent::SessionChanged { session } => {
            entity = Some(scriberig_protocol::EntityReference {
                id: session.to_string(),
            });
            EventKind::SessionChanged
        }
        ApplicationEvent::SessionSelectionChanged {
            session,
            activity_kind,
            summary,
        } => {
            typed_activity_kind = Some(activity_kind);
            entity = Some(scriberig_protocol::EntityReference {
                id: session.to_string(),
            });
            metadata.push(("summary".to_owned(), summary));
            EventKind::SessionSelectionChanged
        }
        ApplicationEvent::PlanActivity {
            session,
            plan,
            node,
            activity_kind,
            state,
            summary,
        } => {
            entity = Some(scriberig_protocol::EntityReference {
                id: plan.to_string(),
            });
            typed_activity_kind = Some(activity_kind);
            typed_activity_state = Some(state);
            metadata.push(("session".to_owned(), session.to_string()));
            if let Some(node) = node {
                metadata.push(("node".to_owned(), node.to_string()));
            }
            metadata.push(("summary".to_owned(), summary));
            EventKind::PlanActivity
        }
        ApplicationEvent::OperationStarted { operation: value } => {
            operation = Some(value);
            EventKind::OperationStarted
        }
        ApplicationEvent::OperationProgressed {
            operation: value,
            progress,
        } => {
            operation = Some(value);
            if let Some(total) = progress.total.filter(|total| *total > 0) {
                let percent = progress
                    .completed
                    .unwrap_or_default()
                    .saturating_mul(100)
                    .checked_div(total)
                    .unwrap_or_default()
                    .min(100);
                metadata.push(("percent".to_owned(), percent.to_string()));
            }
            EventKind::OperationProgressed
        }
        ApplicationEvent::OperationCompleted { operation: value } => {
            operation = Some(value);
            EventKind::OperationCompleted
        }
        ApplicationEvent::OperationFailed {
            operation: value,
            error,
        } => {
            operation = Some(value);
            metadata.push(("error".to_owned(), error));
            EventKind::OperationFailed
        }
        ApplicationEvent::OperationCancelled { operation: value } => {
            operation = Some(value);
            EventKind::OperationCancelled
        }
        ApplicationEvent::SnapshotChanged { snapshot: value } => {
            snapshot = Some(scriberig_protocol::SnapshotReference {
                id: value.to_string(),
            });
            EventKind::SnapshotChanged
        }
        ApplicationEvent::EntityChanged {
            entity: value,
            revision: value_revision,
        } => {
            entity = Some(scriberig_protocol::EntityReference {
                id: value.to_string(),
            });
            revision = Some(scriberig_protocol::RevisionReference {
                entity: scriberig_protocol::EntityReference {
                    id: value.to_string(),
                },
                id: value_revision.to_string(),
            });
            EventKind::EntityChanged
        }
        ApplicationEvent::ProjectionChanged { subject } => {
            entity = Some(scriberig_protocol::EntityReference {
                id: subject.to_string(),
            });
            EventKind::ProjectionChanged
        }
        ApplicationEvent::BranchChanged { branch } => {
            metadata.push(("branch".to_owned(), branch));
            EventKind::BranchChanged
        }
        ApplicationEvent::MigrationCreated { migration } => {
            metadata.push(("migration".to_owned(), migration));
            EventKind::MigrationCreated
        }
        ApplicationEvent::ValidationCompleted { subject, passed } => {
            metadata.push(("subject".to_owned(), subject));
            metadata.push(("passed".to_owned(), passed.to_string()));
            EventKind::ValidationCompleted
        }
        ApplicationEvent::AgentOutputProduced {
            operation: value,
            output,
        } => {
            operation = Some(value);
            metadata.push(("output".to_owned(), output));
            EventKind::AgentOutputProduced
        }
        ApplicationEvent::AgentOutputDelta {
            operation: value,
            text,
        } => {
            operation = Some(value);
            metadata.push(("text".to_owned(), text));
            EventKind::AgentOutputDelta
        }
        ApplicationEvent::AgentActivity {
            operation: value,
            branch,
            activity_kind,
            state,
            summary,
        } => {
            typed_activity_kind = Some(activity_kind);
            typed_activity_state = Some(state);
            operation = Some(value);
            metadata.push(("branch".to_owned(), branch));
            metadata.push(("summary".to_owned(), summary));
            EventKind::AgentActivity
        }
    };
    Event {
        sequence: 0,
        kind,
        entity,
        revision,
        snapshot,
        operation: operation.map(|value| scriberig_protocol::OperationReference {
            id: value.to_string(),
        }),
        activity_kind: typed_activity_kind,
        activity_state: typed_activity_state,
        metadata,
    }
}

#[cfg(test)]
mod tests {
    use super::event_from_application;
    use crate::server::{ApplicationEvent, EventKind};
    use scriberig_core::EntityId;

    /// Maps a session update to a safe refresh event.
    #[test]
    fn session_change_event_carries_only_the_session_reference() {
        let session = EntityId::new();
        let event = event_from_application(ApplicationEvent::SessionChanged { session });

        assert_eq!(event.kind, EventKind::SessionChanged);
        assert_eq!(
            event.entity.expect("session reference").id,
            session.to_string()
        );
        assert!(event.operation.is_none());
        assert_eq!(event.metadata, Vec::new());
    }

    /// Projects safe selection-change summaries without provider configuration.
    #[test]
    fn session_selection_event_carries_safe_activity_metadata() {
        let session = EntityId::new();
        let event = event_from_application(ApplicationEvent::SessionSelectionChanged {
            session,
            activity_kind: scriberig_session::AgentActivityKind::ThinkingLevelChanged,
            summary: "Thinking level selected: careful".to_owned(),
        });

        assert_eq!(event.kind, EventKind::SessionSelectionChanged);
        assert_eq!(
            event.activity_kind,
            Some(scriberig_protocol::SessionActivityKind::ThinkingLevelChanged)
        );
        assert_eq!(
            event.entity.expect("session reference").id,
            session.to_string()
        );
        assert!(event.operation.is_none());
        assert_eq!(
            event.metadata,
            vec![(
                "summary".to_owned(),
                "Thinking level selected: careful".to_owned()
            )]
        );
    }

    /// Projects durable plan activity with stable references and typed state.
    #[test]
    fn plan_activity_event_carries_safe_stable_references() {
        let session = EntityId::from_bytes([7; 16]);
        let plan = EntityId::from_bytes([8; 16]);
        let node = EntityId::from_bytes([9; 16]);
        let event = event_from_application(ApplicationEvent::PlanActivity {
            session,
            plan,
            node: Some(node),
            activity_kind: scriberig_session::SessionActivityKind::PlanNodeCompleted,
            state: scriberig_session::SessionActivityState::Completed,
            summary: "Plan node completed".to_owned(),
        });

        assert_eq!(event.kind, EventKind::PlanActivity);
        assert_eq!(event.entity.expect("plan ref").id, plan.to_string());
        assert_eq!(
            event.activity_kind,
            Some(scriberig_protocol::SessionActivityKind::PlanNodeCompleted)
        );
        assert_eq!(
            event.activity_state,
            Some(scriberig_protocol::SessionActivityState::Completed)
        );
        assert_eq!(
            event.metadata,
            vec![
                ("session".to_owned(), session.to_string()),
                ("node".to_owned(), node.to_string()),
                ("summary".to_owned(), "Plan node completed".to_owned()),
            ]
        );
    }

    /// Projects transient agent status with safe branch and lifecycle metadata.
    #[test]
    fn agent_activity_event_carries_bounded_public_fields() {
        let operation = scriberig_app::OperationId::new("agent-activity-test").expect("operation");
        let event = event_from_application(ApplicationEvent::AgentActivity {
            operation,
            branch: "branch-1".to_owned(),
            activity_kind: scriberig_session::AgentActivityKind::ToolInvocation,
            state: scriberig_session::AgentActivityState::Running,
            summary: "Tool: search".to_owned(),
        });

        assert_eq!(event.kind, EventKind::AgentActivity);
        assert_eq!(
            event.activity_kind,
            Some(scriberig_protocol::SessionActivityKind::ToolInvocation)
        );
        assert_eq!(
            event.activity_state,
            Some(scriberig_protocol::SessionActivityState::Running)
        );
        assert_eq!(
            event.operation.expect("operation reference").id,
            "op_agent-activity-test"
        );
        assert_eq!(
            event.metadata,
            vec![
                ("branch".to_owned(), "branch-1".to_owned()),
                ("summary".to_owned(), "Tool: search".to_owned()),
            ]
        );
    }
}
