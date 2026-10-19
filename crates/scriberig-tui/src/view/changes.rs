//! Defines review projections and explicit confirmation intent.

use scriberig_core::{EntityId, RootName, SnapshotId};

/// Validation state exposed by a change review.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewValidation {
    /// No validation evidence was returned by the application operation.
    Unknown,
    /// The candidate was returned successfully by the application.
    Passed,
    /// The application returned a validation diagnostic.
    Failed(String),
}

/// Read-only review model for an application-owned change or candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeReviewModel {
    /// Exact base snapshot reviewed by the user.
    pub base_snapshot: SnapshotId,
    /// Stable proposed change identity.
    pub change: String,
    /// Optional branch receiving the reviewed candidate.
    pub branch: Option<RootName>,
    /// Affected semantic entities, when the result carries a diff.
    pub affected_entities: Vec<EntityId>,
    /// Structured conflict summaries surfaced by the application.
    pub conflicts: Vec<String>,
    /// Validation state and safe diagnostics.
    pub validation: ReviewValidation,
    /// Registered policy that produced the current validation result.
    pub validation_profile: Option<String>,
    /// Whether the model represents a candidate rather than a promoted state.
    pub requires_confirmation: bool,
}

impl ChangeReviewModel {
    /// Builds a review model from a public candidate summary.
    #[must_use]
    /// - `base_snapshot`: Argument named `base_snapshot`.
    /// - `change`: Argument named `change`.
    /// - `branch`: Argument named `branch`.
    /// - `summary`: Argument named `summary`.
    ///
    /// # Returns
    /// The operation result.
    pub fn from_summary(
        base_snapshot: SnapshotId,
        change: impl Into<String>,
        branch: Option<RootName>,
        summary: &scriberig_app::OperationSummary,
    ) -> Self {
        let validation = match summary.validation {
            Some(scriberig_app::ValidationStatus::Passed) => ReviewValidation::Passed,
            Some(
                scriberig_app::ValidationStatus::Failed | scriberig_app::ValidationStatus::Error,
            ) => ReviewValidation::Failed(if summary.validation_diagnostics.is_empty() {
                "validation failed".to_owned()
            } else {
                summary.validation_diagnostics.join("; ")
            }),
            Some(
                scriberig_app::ValidationStatus::Skipped | scriberig_app::ValidationStatus::Unknown,
            )
            | None => ReviewValidation::Unknown,
        };
        Self {
            base_snapshot,
            change: change.into(),
            branch,
            affected_entities: summary
                .diff
                .as_ref()
                .map(|diff| diff.deltas.iter().map(|delta| delta.entity).collect())
                .unwrap_or_default(),
            conflicts: summary.conflicts.clone(),
            validation,
            validation_profile: summary.validation_profile.clone(),
            requires_confirmation: true,
        }
    }
}

/// Explicit user intent to invoke canonical application commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmedCommit {
    /// Exact reviewed base snapshot.
    pub base_snapshot: SnapshotId,
    /// Stable change identity.
    pub change: String,
    /// Reviewed branch context.
    pub branch: Option<RootName>,
}

#[cfg(test)]
mod tests {
    use super::{ChangeReviewModel, ReviewValidation};
    use scriberig_app::{OperationSummary, ValidationStatus};
    use scriberig_core::SnapshotId;

    /// Verifies review status remains unknown without application evidence.
    #[test]
    fn review_preserves_validation_and_conflict_evidence() {
        let summary = OperationSummary {
            handle: "candidate".to_owned(),
            snapshot: None,
            revision: None,
            branch: None,
            diff: None,
            history: None,
            rendered: None,
            validation: Some(ValidationStatus::Failed),
            validation_profile: Some("rust-workspace".to_owned()),
            validation_diagnostics: vec!["invalid relation".to_owned()],
            conflicts: vec!["entity conflict".to_owned()],
        };
        let model =
            ChangeReviewModel::from_summary(SnapshotId::new([1; 32]), "candidate", None, &summary);
        assert_eq!(
            model.validation,
            ReviewValidation::Failed("invalid relation".to_owned())
        );
        assert_eq!(model.validation_profile.as_deref(), Some("rust-workspace"));
        assert_eq!(model.conflicts, vec!["entity conflict"]);
    }
}
