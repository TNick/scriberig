//! Converts typed remote protocol events to shared TUI application events.

use super::parse_entity;

/// Converts one protocol event into the shared application event model.
///
/// - `event`: Typed remote event envelope.
///
/// The equivalent application event when required fields are valid.
/// # Returns
/// The operation result.
pub(crate) fn map_remote_event(
    event: &scriberig_protocol::Event,
) -> Option<scriberig_app::ApplicationEvent> {
    if event.kind == scriberig_protocol::EventKind::SessionChanged {
        let session = parse_entity(&event.entity.as_ref()?.id).ok()?;
        return Some(scriberig_app::ApplicationEvent::SessionChanged { session });
    }
    if event.kind == scriberig_protocol::EventKind::SessionSelectionChanged {
        let session = parse_entity(&event.entity.as_ref()?.id).ok()?;
        return Some(scriberig_app::ApplicationEvent::SessionSelectionChanged {
            session,
            activity_kind: *event.activity_kind.as_ref()?,
            summary: metadata_value(event, "summary")?,
        });
    }
    if event.kind == scriberig_protocol::EventKind::PlanActivity {
        let plan = parse_entity(&event.entity.as_ref()?.id).ok()?;
        let session = parse_entity(&metadata_value(event, "session")?).ok()?;
        let node = metadata_value(event, "node").and_then(|value| parse_entity(&value).ok());
        return Some(scriberig_app::ApplicationEvent::PlanActivity {
            session,
            plan,
            node,
            activity_kind: *event.activity_kind.as_ref()?,
            state: *event.activity_state.as_ref()?,
            summary: metadata_value(event, "summary")?,
        });
    }
    let operation = event.operation.as_ref()?.id.parse().ok()?;
    match event.kind {
        scriberig_protocol::EventKind::OperationStarted => {
            Some(scriberig_app::ApplicationEvent::OperationStarted { operation })
        }
        scriberig_protocol::EventKind::OperationProgressed => {
            let percent = metadata_value(event, "percent")?.parse::<u8>().ok()?;
            let progress = scriberig_app::OperationProgress {
                status: scriberig_app::OperationStatus::Running,
                completed: Some(u64::from(percent)),
                total: Some(100),
                phase: None,
            };
            Some(scriberig_app::ApplicationEvent::OperationProgressed {
                operation,
                progress,
            })
        }
        scriberig_protocol::EventKind::OperationCompleted => {
            Some(scriberig_app::ApplicationEvent::OperationCompleted { operation })
        }
        scriberig_protocol::EventKind::OperationFailed => {
            let error =
                metadata_value(event, "error").unwrap_or_else(|| "operation failed".to_owned());
            Some(scriberig_app::ApplicationEvent::OperationFailed { operation, error })
        }
        scriberig_protocol::EventKind::OperationCancelled => {
            Some(scriberig_app::ApplicationEvent::OperationCancelled { operation })
        }
        scriberig_protocol::EventKind::AgentOutputProduced => {
            let output = metadata_value(event, "output")?;
            Some(scriberig_app::ApplicationEvent::AgentOutputProduced { operation, output })
        }
        scriberig_protocol::EventKind::AgentOutputDelta => {
            let text = metadata_value(event, "text")?;
            Some(scriberig_app::ApplicationEvent::AgentOutputDelta { operation, text })
        }
        scriberig_protocol::EventKind::AgentActivity => {
            let branch = metadata_value(event, "branch")?;
            let summary = metadata_value(event, "summary")?;
            Some(scriberig_app::ApplicationEvent::AgentActivity {
                operation,
                branch,
                activity_kind: *event.activity_kind.as_ref()?,
                state: *event.activity_state.as_ref()?,
                summary,
            })
        }
        _ => None,
    }
}

/// Reads one string value from safe event metadata.
///
/// - `event`: Protocol event containing the metadata.
/// - `key`: Stable metadata key.
///
/// The metadata value when present.
/// # Returns
/// The operation result.
fn metadata_value(event: &scriberig_protocol::Event, key: &str) -> Option<String> {
    event
        .metadata
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
}
