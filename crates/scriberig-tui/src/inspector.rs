//! Defines the TUI inspector projection and safe presentation errors.

use scriberig_app::{ApplicationError, InspectResult, TrustSummary};
use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Explicit loading state for an effect-backed inspector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoadState<T> {
    /// No request has been issued.
    NotRequested,
    /// A request is currently outstanding.
    Loading {
        /// Monotonic request token used to reject stale responses.
        request_id: u64,
    },
    /// A request completed successfully.
    Ready(T),
    /// A request failed with a safe category and message.
    Failed {
        /// Safe structured error category.
        category: String,
        /// Safe user-facing error message.
        message: String,
        /// Whether the previous successful data remains visible.
        stale: bool,
    },
}

/// Presentation-only inspection data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectorModel {
    /// Conceptual entity identity.
    pub entity: EntityId,
    /// Exact revision identity, when resolved.
    pub revision: Option<RevisionId>,
    /// Exact snapshot context, when resolved.
    pub snapshot: Option<SnapshotId>,
    /// Schema/object kind summary.
    pub kind: Option<String>,
    /// Short semantic label.
    pub label: Option<String>,
    /// Safe short description.
    pub description: Option<String>,
    /// Public follow-up capability identifiers.
    pub capabilities: Vec<String>,
    /// Trust/provenance summary when supplied by the application.
    pub trust: Option<TrustSummary>,
}

impl InspectorModel {
    /// Builds a safe TUI projection from a public inspection result.
    #[must_use]
    /// - `result`: Argument named `result`.
    ///
    /// # Returns
    /// The operation result.
    pub fn from_result(result: InspectResult) -> Self {
        Self {
            entity: result.entity,
            revision: result.revision,
            snapshot: result.snapshot,
            kind: result.object_type.map(|value| sanitize(&value.to_string())),
            label: result.label.map(|value| sanitize(&value)),
            description: result.description.map(|value| sanitize(&value)),
            capabilities: result
                .capabilities
                .iter()
                .map(|value| sanitize(value))
                .collect(),
            trust: result.trust,
        }
    }
}

/// Converts an application error into a distinct safe presentation state.
#[must_use]
/// - `error`: Argument named `error`.
/// - `stale`: Argument named `stale`.
///
/// # Returns
/// The operation result.
pub fn presentation_error(error: &ApplicationError, stale: bool) -> LoadState<InspectorModel> {
    LoadState::Failed {
        category: error.category().to_owned(),
        message: sanitize(&error.to_string()),
        stale,
    }
}

/// Removes terminal control characters from external text.
#[must_use]
/// - `text`: Argument named `text`.
///
/// # Returns
/// The operation result.
pub fn sanitize(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_control())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{InspectorModel, sanitize};
    use scriberig_app::InspectResult;
    use scriberig_core::{EntityId, RevisionId, SnapshotId};

    /// Verifies exact identity context remains distinct in the projection.
    #[test]
    fn preserves_exact_identity_context() {
        let model = InspectorModel::from_result(InspectResult {
            entity: EntityId::from_bytes([1; 16]),
            revision: Some(RevisionId::new([2; 32])),
            snapshot: Some(SnapshotId::new([3; 32])),
            object_type: None,
            label: Some("label\nunsafe".to_owned()),
            description: None,
            capabilities: Vec::new(),
            trust: None,
        });
        assert_eq!(model.revision, Some(RevisionId::new([2; 32])));
        assert_eq!(model.snapshot, Some(SnapshotId::new([3; 32])));
        assert_eq!(model.label.as_deref(), Some("labelunsafe"));
    }

    /// Verifies terminal control characters are removed from external text.
    #[test]
    fn sanitizes_control_characters() {
        assert_eq!(sanitize("a\u{1b}[31mb"), "a[31mb");
    }
}
