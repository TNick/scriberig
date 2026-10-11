//! Projects structured semantic diff results into stable terminal rows.

use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// One semantic entity transition in a diff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffRow {
    /// Stable entity identity.
    pub entity: EntityId,
    /// Revision on the left/base side.
    pub before: Option<RevisionId>,
    /// Revision on the right/target side.
    pub after: Option<RevisionId>,
}

/// Explicit two-sided semantic diff view model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffModel {
    /// Explicit left/base snapshot.
    pub left: SnapshotId,
    /// Explicit right/target snapshot.
    pub right: SnapshotId,
    /// Structured changed entity rows.
    pub rows: Vec<DiffRow>,
}

impl DiffModel {
    /// Builds a TUI model without comparing rendered text.
    #[must_use]
    pub fn from_diff(diff: &sr_change::SemanticDiff) -> Self {
        Self {
            left: diff.left,
            right: diff.right,
            rows: diff
                .deltas
                .iter()
                .map(|delta| DiffRow {
                    entity: delta.entity,
                    before: delta.before,
                    after: delta.after,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DiffModel;
    use scriberig_core::{EntityId, RevisionId, SnapshotId};
    use sr_change::{EntityDelta, SemanticDiff};

    /// Verifies both exact diff sides and stable row references are retained.
    #[test]
    fn preserves_explicit_sides_and_entity_reference() {
        let entity = EntityId::from_bytes([1; 16]);
        let diff = SemanticDiff::new(
            SnapshotId::new([2; 32]),
            SnapshotId::new([3; 32]),
            vec![
                EntityDelta::new(
                    entity,
                    Some(RevisionId::new([4; 32])),
                    Some(RevisionId::new([5; 32])),
                )
                .expect("delta"),
            ],
        );
        let model = DiffModel::from_diff(&diff);
        assert_eq!(model.left, SnapshotId::new([2; 32]));
        assert_eq!(model.right, SnapshotId::new([3; 32]));
        assert_eq!(model.rows[0].entity, entity);
    }
}
