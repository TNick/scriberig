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
    /// The value produced by the declaration.
    pub fn from_summary(
        base_snapshot: SnapshotId,
        change: impl Into<String>,
        branch: Option<RootName>,
        summary: &scriberig_app::OperationSummary,
    ) -> Self {
        Self {
            base_snapshot,
            change: change.into(),
            branch,
            affected_entities: summary
                .diff
                .as_ref()
                .map(|diff| diff.deltas.iter().map(|delta| delta.entity).collect())
                .unwrap_or_default(),
            conflicts: Vec::new(),
            validation: ReviewValidation::Passed,
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
