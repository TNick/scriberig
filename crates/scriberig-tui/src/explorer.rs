//! Defines semantic explorer rows without inventing unsupported relation trees.

use crate::selection::EntitySelection;
use scriberig_core::{EntityId, SnapshotId};
use scriberig_query::ContinuationToken;

/// One explorer row backed by a stable semantic selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerRow {
    /// Stable row selection.
    pub selection: EntitySelection,
    /// Safe compact label.
    pub label: String,
    /// Semantic kind shown alongside the human-readable label.
    pub kind: Option<String>,
    /// Whether the row can be activated for inspection.
    pub inspectable: bool,
}

#[cfg(test)]
mod tests {
    use super::ExplorerModel;
    use crate::selection::EntitySelection;
    use scriberig_core::{EntityId, SnapshotId};

    /// Verifies composition-supplied children retain exact snapshot context.
    #[test]
    fn adds_snapshot_bound_children() {
        let snapshot = SnapshotId::new([9; 32]);
        let root = EntityId::from_bytes([1; 16]);
        let child = EntityId::from_bytes([2; 16]);
        let mut model = ExplorerModel::from_selection(EntitySelection {
            entity: root,
            revision: None,
            snapshot: Some(snapshot),
        });

        model.add_entities([child], Some(snapshot));

        assert_eq!(model.rows[0].selection.entity, child);
        assert_eq!(model.rows[0].selection.snapshot, Some(snapshot));
    }
}

/// Explorer model containing only explicitly known semantic rows.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExplorerModel {
    /// Optional semantic root row.
    pub root: Option<ExplorerRow>,
    /// Rows returned by a future public list/follow operation.
    pub rows: Vec<ExplorerRow>,
    /// Opaque continuation for the next bounded child page.
    pub next: Option<ContinuationToken>,
}

impl ExplorerModel {
    /// Creates an explorer rooted at an explicitly entered entity.
    #[must_use]
    /// - `selection`: Argument named `selection`.
    ///
    /// # Returns
    /// The operation result.
    pub fn from_selection(selection: EntitySelection) -> Self {
        let label = selection.entity.to_string();
        Self {
            root: Some(ExplorerRow {
                selection,
                label,
                kind: None,
                inspectable: true,
            }),
            rows: Vec::new(),
            next: None,
        }
    }

    /// Adds bounded, composition-supplied rows to the current semantic root.
    ///
    /// - `self`: Explorer model receiving the rows.
    /// - `entities`: Stable entities visible in the same snapshot.
    /// - `snapshot`: Exact snapshot context for every row.
    pub fn add_entities(
        &mut self,
        entities: impl IntoIterator<Item = EntityId>,
        snapshot: Option<SnapshotId>,
    ) {
        self.rows
            .extend(entities.into_iter().map(|entity| ExplorerRow {
                selection: EntitySelection {
                    entity,
                    revision: None,
                    snapshot,
                },
                label: entity.to_string(),
                kind: None,
                inspectable: true,
            }));
    }
}

/// One bounded Explorer page returned by the application service.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExplorerPage {
    /// Rows returned for this page.
    pub rows: Vec<ExplorerRow>,
    /// Opaque continuation for a subsequent page.
    pub next: Option<ContinuationToken>,
}
