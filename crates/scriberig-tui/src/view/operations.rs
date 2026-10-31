//! Projects shared operation lifecycle records into terminal presentation.

use scriberig_app::{OperationRecord, OperationStatus};
use scriberig_core::OperationId;

/// A bounded presentation model for one application operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationModel {
    /// Stable operation identity.
    pub id: OperationId,
    /// Shared operation family label.
    pub kind: String,
    /// Current shared lifecycle state.
    pub status: OperationStatus,
    /// Optional progress label.
    pub progress: Option<String>,
    /// Stable result handles returned by the application.
    pub result_handles: Vec<String>,
    /// Safe structured failure category, when available.
    pub error: Option<String>,
}

impl OperationModel {
    /// Builds a terminal model without copying backend-specific state.
    #[must_use]
    /// - `record`: Argument named `record`.
    ///
    /// # Returns
    /// The operation result.
    pub fn from_record(record: &OperationRecord) -> Self {
        Self {
            id: record.reference.clone(),
            kind: format!("{:?}", record.kind),
            status: record.status,
            progress: record.progress.as_ref().and_then(|progress| {
                progress.phase.clone().or_else(|| {
                    progress
                        .completed
                        .zip(progress.total)
                        .map(|(done, total)| format!("{done}/{total}"))
                })
            }),
            result_handles: record.result_handles.clone(),
            error: record.error.as_ref().map(ToString::to_string),
        }
    }

    /// Applies a lifecycle event while preserving details from an earlier observation.
    ///
    /// - `event`: Shared lifecycle event for one operation.
    /// - `previous`: Earlier operation details, when already observed.
    ///
    /// An updated terminal model for operation lifecycle events.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn from_event(
        event: &scriberig_app::ApplicationEvent,
        previous: Option<&Self>,
    ) -> Option<Self> {
        let (id, status, progress, error) = match event {
            scriberig_app::ApplicationEvent::OperationStarted { operation } => {
                (operation, OperationStatus::Running, None, None)
            }
            scriberig_app::ApplicationEvent::OperationProgressed {
                operation,
                progress,
            } => (
                operation,
                OperationStatus::Running,
                progress.phase.clone().or_else(|| {
                    progress
                        .completed
                        .zip(progress.total)
                        .map(|(done, total)| format!("{done}/{total}"))
                }),
                None,
            ),
            scriberig_app::ApplicationEvent::OperationCompleted { operation } => {
                (operation, OperationStatus::Completed, None, None)
            }
            scriberig_app::ApplicationEvent::OperationFailed { operation, error } => (
                operation,
                OperationStatus::Failed,
                None,
                Some(error.clone()),
            ),
            scriberig_app::ApplicationEvent::OperationCancelled { operation } => {
                (operation, OperationStatus::Cancelled, None, None)
            }
            _ => return None,
        };
        Some(Self {
            id: id.clone(),
            kind: previous.map_or_else(|| "Operation".to_owned(), |value| value.kind.clone()),
            status,
            progress: progress.or_else(|| previous.and_then(|value| value.progress.clone())),
            result_handles: previous.map_or_else(Vec::new, |value| value.result_handles.clone()),
            error: error.or_else(|| previous.and_then(|value| value.error.clone())),
        })
    }

    /// Reports whether this operation may still be cancelled.
    #[must_use]
    pub const fn cancellable(&self) -> bool {
        matches!(
            self.status,
            OperationStatus::Queued | OperationStatus::Running
        )
    }
}

#[cfg(test)]
mod tests {
    use super::OperationModel;
    use scriberig_app::{OperationKind, OperationRecord, OperationStatus};
    use scriberig_core::OperationId;

    /// Verifies operation presentation preserves shared lifecycle identity.
    #[test]
    fn model_preserves_identity_and_status() {
        let record = OperationRecord::queued(
            OperationId::new("test").expect("operation id"),
            OperationKind::Query,
        );
        let model = OperationModel::from_record(&record);
        assert_eq!(model.id.to_string(), "op_test");
        assert_eq!(model.status, OperationStatus::Queued);
        assert!(model.cancellable());
    }

    /// Verifies progress text is projected only from shared lifecycle state.
    #[test]
    fn progress_is_projected_only_from_shared_operation_state() {
        let mut reported = OperationRecord::queued(
            OperationId::new("reported").expect("operation id"),
            OperationKind::Agent,
        );
        reported.status = OperationStatus::Running;
        reported.progress = Some(scriberig_app::OperationProgress {
            status: OperationStatus::Running,
            completed: Some(3),
            total: Some(7),
            phase: Some("scanning sources".to_owned()),
        });
        let mut counted = OperationRecord::queued(
            OperationId::new("counted").expect("operation id"),
            OperationKind::Agent,
        );
        counted.status = OperationStatus::Running;
        counted.progress = Some(scriberig_app::OperationProgress {
            status: OperationStatus::Running,
            completed: Some(3),
            total: Some(7),
            phase: None,
        });
        let silent = OperationRecord::queued(
            OperationId::new("silent").expect("operation id"),
            OperationKind::Agent,
        );

        let reported = OperationModel::from_record(&reported);
        let counted = OperationModel::from_record(&counted);
        let silent = OperationModel::from_record(&silent);

        assert_eq!(reported.progress.as_deref(), Some("scanning sources"));
        assert_eq!(counted.progress.as_deref(), Some("3/7"));
        assert_eq!(silent.progress, None, "an idle record gains no progress");
    }

    /// Verifies a terminal lifecycle event never invents progress or result state.
    #[test]
    fn terminal_lifecycle_events_do_not_fabricate_operation_state() {
        let completed = OperationModel::from_event(
            &scriberig_app::ApplicationEvent::OperationCompleted {
                operation: OperationId::new("bare").expect("operation id"),
            },
            None,
        )
        .expect("completed lifecycle event");

        assert_eq!(completed.status, OperationStatus::Completed);
        assert_eq!(completed.progress, None);
        assert_eq!(
            completed.result_handles,
            Vec::<String>::new(),
            "a terminal event with no earlier observation gains no result handles"
        );
        assert_eq!(
            completed.kind, "Operation",
            "no local operation family is invented"
        );
    }
}
