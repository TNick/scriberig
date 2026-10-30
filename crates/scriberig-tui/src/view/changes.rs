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
    /// Candidate result snapshot, when returned by the operation.
    pub candidate_snapshot: Option<SnapshotId>,
    /// Stable proposed change identity.
    pub change: String,
    /// Optional branch receiving the reviewed candidate.
    pub branch: Option<RootName>,
    /// Affected semantic entities, when the result carries a diff.
    pub affected_entities: Vec<EntityId>,
    /// Exact shared entity and revision transitions returned by the diff.
    pub deltas: Vec<sr_change::EntityDelta>,
    /// Non-blocking candidate and materialization details.
    pub details: Vec<String>,
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
            candidate_snapshot: summary.snapshot,
            change: change.into(),
            branch,
            affected_entities: summary
                .diff
                .as_ref()
                .map(|diff| diff.deltas.iter().map(|delta| delta.entity).collect())
                .unwrap_or_default(),
            deltas: summary
                .diff
                .as_ref()
                .map_or_else(Vec::new, |diff| diff.deltas.clone()),
            details: summary.review_details.clone(),
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
    /// Exact candidate result snapshot shown during review.
    pub candidate_snapshot: SnapshotId,
    /// Stable change identity.
    pub change: String,
    /// Reviewed branch context.
    pub branch: Option<RootName>,
}

#[cfg(test)]
mod tests {
    use super::{ChangeReviewModel, ReviewValidation};
    use scriberig_app::{OperationSummary, ValidationStatus};
    use scriberig_core::{EntityId, RevisionId, SnapshotId};

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
            review_details: Vec::new(),
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

    /// Verifies the exact candidate snapshot survives adaptation for review.
    #[test]
    fn review_preserves_candidate_snapshot_identity() {
        let candidate = SnapshotId::new([2; 32]);
        let summary = OperationSummary {
            handle: "candidate".to_owned(),
            snapshot: Some(candidate),
            revision: None,
            branch: None,
            diff: None,
            history: None,
            rendered: None,
            validation: Some(ValidationStatus::Passed),
            validation_profile: None,
            validation_diagnostics: Vec::new(),
            review_details: Vec::new(),
            conflicts: Vec::new(),
        };
        let model =
            ChangeReviewModel::from_summary(SnapshotId::new([1; 32]), "candidate", None, &summary);

        assert_eq!(model.candidate_snapshot, Some(candidate));
    }

    /// Verifies exact shared before/after revision references survive review.
    #[test]
    fn review_preserves_semantic_revision_deltas() {
        let base = SnapshotId::new([3; 32]);
        let candidate = SnapshotId::new([4; 32]);
        let entity = EntityId::from_bytes([5; 16]);
        let before = RevisionId::new([6; 32]);
        let after = RevisionId::new([7; 32]);
        let summary = OperationSummary {
            handle: "candidate".to_owned(),
            snapshot: Some(candidate),
            revision: None,
            branch: None,
            diff: Some(sr_change::SemanticDiff::new(
                base,
                candidate,
                vec![
                    sr_change::EntityDelta::new(entity, Some(before), Some(after))
                        .expect("changed revision"),
                ],
            )),
            history: None,
            rendered: None,
            validation: Some(ValidationStatus::Passed),
            validation_profile: None,
            validation_diagnostics: Vec::new(),
            review_details: Vec::new(),
            conflicts: Vec::new(),
        };
        let model = ChangeReviewModel::from_summary(base, "candidate", None, &summary);

        assert_eq!(model.deltas.len(), 1);
        assert_eq!(model.deltas[0].entity, entity);
        assert_eq!(model.deltas[0].before, Some(before));
        assert_eq!(model.deltas[0].after, Some(after));
    }

    /// Keeps candidate presentation details separate from blocking conflicts.
    #[test]
    fn review_details_do_not_block_explicit_confirmation() {
        let summary = OperationSummary {
            handle: "candidate".to_owned(),
            snapshot: None,
            revision: None,
            branch: None,
            diff: None,
            history: None,
            rendered: None,
            validation: Some(ValidationStatus::Passed),
            validation_profile: None,
            validation_diagnostics: Vec::new(),
            review_details: vec!["materialized src/lib.rs".to_owned()],
            conflicts: Vec::new(),
        };

        let model =
            ChangeReviewModel::from_summary(SnapshotId::new([8; 32]), "candidate", None, &summary);

        assert_eq!(model.details, vec!["materialized src/lib.rs"]);
        assert_eq!(model.conflicts, Vec::<String>::new());
    }
}
