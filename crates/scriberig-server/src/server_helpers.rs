//! Converts application results and events into protocol values.

use crate::server::{ApplicationEvent, Event, EventKind, Operation};

/// Extracts stable result handles without exposing canonical result payloads.
/// - `super`: Parent-module imports used by this helper.
/// - `result`: Argument named `result`.
///
/// # Returns
/// The value produced by the declaration.
pub(super) fn result_handles(result: &scriberig_app::ApplicationResult) -> Vec<String> {
    let summary = match result {
        scriberig_app::ApplicationResult::Diff(value)
        | scriberig_app::ApplicationResult::History(value)
        | scriberig_app::ApplicationResult::Render(value)
        | scriberig_app::ApplicationResult::Branch(value)
        | scriberig_app::ApplicationResult::Apply(value)
        | scriberig_app::ApplicationResult::Commit(value) => Some(value),
        _ => None,
    };
    summary.map_or_else(Vec::new, |value| vec![value.handle.clone()])
}

/// Tests whether a protocol event matches a case-insensitive kind filter.
/// - `super`: Parent-module imports used by this helper.
///
/// # Parameters
/// - `event`: Event whose kind is being tested.
/// - `event_kinds`: Requested event kind names; empty selects every event.
///
/// # Returns
/// `true` when the event is selected by the subscription.
pub(super) fn event_matches(event: &Event, event_kinds: &[String]) -> bool {
    event_kinds.is_empty()
        || event_kinds.iter().any(|kind| {
            kind.eq_ignore_ascii_case(&format!("{:?}", event.kind))
                || kind.eq_ignore_ascii_case(&format!("{:?}", event.kind).replace('_', ""))
        })
}

/// Converts an application operation record to its public lifecycle form.
/// - `super`: Parent-module imports used by this helper.
/// - `record`: Argument named `record`.
///
/// # Returns
/// The value produced by the declaration.
pub(super) fn operation_from_record(record: scriberig_app::OperationRecord) -> Operation {
    Operation {
        reference: scriberig_protocol::OperationReference {
            id: record.reference.to_string(),
        },
        kind: match record.kind {
            scriberig_app::OperationKind::Inspect => {
                scriberig_protocol::operation::OperationKind::Inspect
            }
            scriberig_app::OperationKind::Query => {
                scriberig_protocol::operation::OperationKind::Query
            }
            scriberig_app::OperationKind::Apply => {
                scriberig_protocol::operation::OperationKind::Apply
            }
            scriberig_app::OperationKind::List => {
                scriberig_protocol::operation::OperationKind::List
            }
            scriberig_app::OperationKind::Follow => {
                scriberig_protocol::operation::OperationKind::Follow
            }
            scriberig_app::OperationKind::Find => {
                scriberig_protocol::operation::OperationKind::Find
            }
            scriberig_app::OperationKind::Diff => {
                scriberig_protocol::operation::OperationKind::Diff
            }
            scriberig_app::OperationKind::History => {
                scriberig_protocol::operation::OperationKind::History
            }
            scriberig_app::OperationKind::Render => {
                scriberig_protocol::operation::OperationKind::Render
            }
            scriberig_app::OperationKind::Branch => {
                scriberig_protocol::operation::OperationKind::Branch
            }
            scriberig_app::OperationKind::Commit => {
                scriberig_protocol::operation::OperationKind::Commit
            }
        },
        status: match record.status {
            scriberig_app::OperationStatus::Queued => {
                scriberig_protocol::operation::OperationStatus::Queued
            }
            scriberig_app::OperationStatus::Running => {
                scriberig_protocol::operation::OperationStatus::Running
            }
            scriberig_app::OperationStatus::Completed => {
                scriberig_protocol::operation::OperationStatus::Completed
            }
            scriberig_app::OperationStatus::Failed => {
                scriberig_protocol::operation::OperationStatus::Failed
            }
            scriberig_app::OperationStatus::Cancelled => {
                scriberig_protocol::operation::OperationStatus::Cancelled
            }
        },
        progress: record.progress.map(|progress| {
            let percent = progress
                .total
                .filter(|total| *total > 0)
                .map_or(0, |total| {
                    progress
                        .completed
                        .unwrap_or_default()
                        .saturating_mul(100)
                        .checked_div(total)
                        .unwrap_or(0)
                        .min(100) as u8
                });
            scriberig_protocol::OperationProgress {
                percent,
                message: progress.phase,
            }
        }),
        result_snapshot: None,
        result_handles: record.result_handles,
        error: record.error.map(|error| error.to_string()),
    }
}

/// Converts one application lifecycle event to a stable protocol envelope.
/// - `super`: Parent-module imports used by this helper.
/// - `event`: Argument named `event`.
///
/// # Returns
/// The value produced by the declaration.
pub(super) fn event_from_application(event: ApplicationEvent) -> Event {
    let mut entity = None;
    let mut revision = None;
    let mut snapshot = None;
    let mut operation = None;
    let mut metadata = Vec::new();
    let kind = match event {
        ApplicationEvent::OperationStarted { operation: value } => {
            operation = Some(value);
            EventKind::OperationStarted
        }
        ApplicationEvent::OperationProgressed {
            operation: value, ..
        } => {
            operation = Some(value);
            EventKind::OperationProgressed
        }
        ApplicationEvent::OperationCompleted { operation: value } => {
            operation = Some(value);
            EventKind::OperationCompleted
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
        ApplicationEvent::AgentOutputProduced { output } => {
            metadata.push(("output".to_owned(), output));
            EventKind::AgentOutputProduced
        }
    };
    Event {
        kind,
        entity,
        revision,
        snapshot,
        operation: operation.map(|value| scriberig_protocol::OperationReference {
            id: value.to_string(),
        }),
        metadata,
    }
}
