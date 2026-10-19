//! Provides storage-operation helper queries.

#![allow(clippy::wildcard_imports)]

use super::*;

impl<S> StoreOperationProvider<S> {
    /// Derives migration references from durable snapshot ancestry.
    ///
    /// - `self`: Storage-backed operation provider.
    /// - `snapshot`: Exact snapshot whose ancestry should be projected.
    /// - `super`: Parent-module visibility marker.
    ///
    /// A deterministic history graph containing each reachable snapshot edge.
    ///
    /// Returns a snapshot-unavailable error when ancestry or diff data cannot
    /// be read.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn history_for_snapshot(
        &self,
        snapshot: scriberig_core::SnapshotId,
    ) -> Result<HistoryGraph, ApplicationError>
    where
        S: SnapshotStore,
    {
        let mut graph = HistoryGraph::new();
        let mut visited = BTreeSet::new();
        let mut pending = VecDeque::from([snapshot]);
        while let Some(result) = pending.pop_front() {
            if !visited.insert(result) {
                continue;
            }
            let parents = self
                .store
                .parents(result)
                .map_err(|_| ApplicationError::SnapshotUnavailable { snapshot: result })?;
            for parent in parents {
                let changes = self
                    .store
                    .diff_snapshot_revisions(parent, result)
                    .map_err(|_| ApplicationError::SnapshotUnavailable { snapshot: result })?;
                let deltas = changes
                    .into_iter()
                    .filter_map(|change| {
                        let (entity, before, after) = match change {
                            SnapshotDiff::Added { entity, revision } => {
                                (entity, None, Some(revision))
                            }
                            SnapshotDiff::Removed { entity, revision } => {
                                (entity, Some(revision), None)
                            }
                            SnapshotDiff::Changed { entity, from, to } => {
                                (entity, Some(from), Some(to))
                            }
                        };
                        EntityDelta::new(entity, before, after).ok()
                    })
                    .collect();
                graph.insert(MigrationReference {
                    id: format!("snapshot:{result}"),
                    base: parent,
                    result,
                    parents: vec![format!("snapshot:{parent}")],
                    deltas,
                    workspace: None,
                    source_change_id: self
                        .store
                        .change_provenance(result)
                        .map_err(|_| ApplicationError::SnapshotUnavailable { snapshot: result })?
                        .map(|provenance| provenance.change_id),
                });
                pending.push_back(parent);
            }
        }
        Ok(graph)
    }

    /// Resolves a registered migration into storage changes.
    ///
    /// - `self`: Provider owning canonical migrations.
    /// - `id`: Stable migration handle.
    /// - `super`: Parent-module visibility marker.
    ///
    /// Exact storage changes in canonical entity order.
    ///
    /// Returns not-found when the migration handle is unknown.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn changes_for(&self, id: &str) -> Result<Vec<SnapshotChange>, ApplicationError> {
        let migrations = self
            .migrations
            .lock()
            .map_err(|_| ApplicationError::Internal {
                message: "migration registry lock failed".to_owned(),
            })?;
        let migration = migrations
            .get(id)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: id.to_owned(),
            })?;
        Ok(migration
            .deltas
            .iter()
            .map(|delta| match delta.after {
                Some(revision) => SnapshotChange::Set(delta.entity, revision),
                None => SnapshotChange::Remove(delta.entity),
            })
            .collect())
    }
}

/// Creates a stable unsupported-operation error.
///
/// - `operation`: Operation name that lacks a configured provider.
///
/// Structured unsupported-operation failure.
/// # Returns
/// The operation result.
pub(super) fn unsupported(operation: &str) -> ApplicationError {
    ApplicationError::UnsupportedOperation {
        operation: operation.to_owned(),
    }
}

/// Maps optimistic root failures without leaking backend details.
///
/// - `error`: Root-store failure to translate.
///
/// Structured application conflict or internal failure.
/// # Returns
/// The operation result.
pub(super) fn map_root_error(error: &RootError) -> ApplicationError {
    match error {
        RootError::Conflict {
            expected, actual, ..
        } => ApplicationError::Conflict {
            subject: "main".to_owned(),
            details: Box::new(crate::ConflictDetails {
                left: expected.map(|value| value.to_string()),
                right: actual.map(|value| value.to_string()),
                ..crate::ConflictDetails::default()
            }),
        },
        RootError::Backend(_) => ApplicationError::Internal {
            message: "root promotion failed".to_owned(),
        },
    }
}
