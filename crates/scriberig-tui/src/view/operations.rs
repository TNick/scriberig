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
    /// The value produced by the declaration.
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
}
