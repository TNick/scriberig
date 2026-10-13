//! Projects application history results into stable terminal rows.

use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// One immutable migration row in the history view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryRow {
    /// Stable migration identity.
    pub id: String,
    /// Exact base snapshot.
    pub base: SnapshotId,
    /// Exact result snapshot.
    pub result: SnapshotId,
    /// Stable changed entities in this migration.
    pub entities: Vec<(EntityId, Option<RevisionId>)>,
    /// Optional workspace or branch provenance.
    pub workspace: Option<String>,
    /// Accepted project source change that produced the result snapshot.
    pub source_change_id: Option<String>,
}

/// Bounded history projection for one exact snapshot context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryModel {
    /// Snapshot whose retained history was requested.
    pub snapshot: SnapshotId,
    /// Deterministically ordered migration rows.
    pub rows: Vec<HistoryRow>,
}

impl HistoryModel {
    /// Builds a TUI history model from a public application summary.
    #[must_use]
    /// - `snapshot`: Argument named `snapshot`.
    /// - `summary`: Argument named `summary`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn from_summary(snapshot: SnapshotId, summary: &scriberig_app::OperationSummary) -> Self {
        let rows = summary
            .history
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|migration| HistoryRow {
                id: migration.id.clone(),
                base: migration.base,
                result: migration.result,
                entities: migration
                    .deltas
                    .iter()
                    .map(|delta| (delta.entity, delta.after.or(delta.before)))
                    .collect(),
                workspace: migration.workspace.clone(),
                source_change_id: migration.source_change_id.clone(),
            })
            .collect();
        Self { snapshot, rows }
    }

    /// Returns an exact historical selection for a changed entity row.
    #[must_use]
    /// - `row`: Argument named `row`.
    /// - `entity`: Argument named `entity`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn selection(
        &self,
        row: usize,
        entity: usize,
    ) -> Option<(EntityId, Option<RevisionId>, SnapshotId)> {
        let migration = self.rows.get(row)?;
        let (selected, revision) = migration.entities.get(entity).copied()?;
        Some((selected, revision, migration.result))
    }
}

#[cfg(test)]
mod tests {
    use super::HistoryModel;
    use scriberig_core::{EntityId, RevisionId, SnapshotId};
    use sr_change::{EntityDelta, MigrationReference};

    /// Verifies history activation preserves the exact result revision context.
    #[test]
    fn activation_keeps_historical_revision_and_snapshot() {
        let entity = EntityId::from_bytes([1; 16]);
        let revision = RevisionId::new([2; 32]);
        let snapshot = SnapshotId::new([3; 32]);
        let mut summary = scriberig_app::OperationSummary {
            handle: "history".to_owned(),
            snapshot: Some(snapshot),
            revision: None,
            branch: None,
            diff: None,
            history: Some(vec![MigrationReference {
                id: "m1".to_owned(),
                base: SnapshotId::new([4; 32]),
                result: snapshot,
                parents: Vec::new(),
                deltas: vec![EntityDelta::new(entity, None, Some(revision)).expect("delta")],
                workspace: None,
                source_change_id: Some("candidate-php".to_owned()),
            }]),
            rendered: None,
            validation: None,
            validation_profile: None,
            validation_diagnostics: Vec::new(),
            conflicts: Vec::new(),
        };
        let model = HistoryModel::from_summary(snapshot, &summary);
        assert_eq!(
            model.rows[0].source_change_id.as_deref(),
            Some("candidate-php")
        );
        assert_eq!(
            model.selection(0, 0),
            Some((entity, Some(revision), snapshot))
        );
        summary.history = None;
    }
}
