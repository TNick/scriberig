//! Provides storage-operation helper queries.

#![allow(clippy::wildcard_imports)]

use super::*;

impl<S> StoreOperationProvider<S> {
    /// Registers a canonical migration for later apply or commit execution.
    ///
    /// # Parameters
    /// - `self`: Provider owning the migration registry.
    /// - `id`: Stable migration handle supplied by the caller.
    /// - `migration`: Validated canonical migration.
    ///
    /// # Returns
    /// `true` when the handle was new, or `false` when it already existed.
    #[must_use]
    pub fn register_migration(
        &self,
        id: impl Into<String>,
        migration: sr_change::Migration,
    ) -> bool {
        let Ok(mut migrations) = self.migrations.lock() else {
            return false;
        };
        migrations.insert(id.into(), migration).is_none()
    }

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
                let provenance = self
                    .store
                    .change_provenance(result)
                    .map_err(|_| ApplicationError::SnapshotUnavailable { snapshot: result })?;
                graph.insert(MigrationReference {
                    id: format!("snapshot:{result}"),
                    base: parent,
                    result,
                    parents: vec![format!("snapshot:{parent}")],
                    deltas,
                    workspace: None,
                    source_change_id: provenance.as_ref().map(|value| value.change_id.clone()),
                    semantic_operation: provenance.and_then(|value| value.semantic_operation),
                });
                pending.push_back(parent);
            }
        }
        Ok(graph)
    }

    /// Resolves a registered migration into atomic entity and relation changes.
    ///
    /// - `self`: Provider owning canonical migrations.
    /// - `id`: Stable migration handle.
    /// - `super`: Parent-module visibility marker.
    ///
    /// Exact entity and typed relation changes from the migration.
    ///
    /// Returns not-found when the migration handle is unknown.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn semantic_changes_for(
        &self,
        id: &str,
    ) -> Result<(Vec<SnapshotChange>, Vec<SnapshotRelationChange>), ApplicationError> {
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
        let entity_changes = migration
            .deltas
            .iter()
            .map(|delta| match delta.after {
                Some(revision) => SnapshotChange::Set(delta.entity, revision),
                None => SnapshotChange::Remove(delta.entity),
            })
            .collect();
        let relation_changes = migration
            .operations
            .iter()
            .filter_map(|operation| match operation {
                sr_change::Operation::AddRelation { from, relation, to } => {
                    Some((*from, relation, *to, true))
                }
                sr_change::Operation::RemoveRelation { from, relation, to } => {
                    Some((*from, relation, *to, false))
                }
                _ => None,
            })
            .map(|(from, relation, to, add)| {
                let relation_type =
                    scriberig_core::TypeId::new(relation.clone()).map_err(|error| {
                        ApplicationError::InvalidRequest {
                            operation: "migration relation".to_owned(),
                            message: error.to_string(),
                        }
                    })?;
                let relation = scriberig_store::SnapshotRelation::new(from, relation_type, to);
                Ok(if add {
                    SnapshotRelationChange::Add(relation)
                } else {
                    SnapshotRelationChange::Remove(relation)
                })
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;
        Ok((entity_changes, relation_changes))
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
