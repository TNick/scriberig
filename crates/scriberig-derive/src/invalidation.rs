//! Identifies records affected by explicitly declared semantic dependency changes.

use crate::dependency::Dependency;
use crate::result::DerivedResultRecord;
use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Describes one semantic dependency changing from an exact old value to new.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticChange {
    /// Exact entity revision changed.
    Revision {
        /// Entity whose revision changed.
        entity: EntityId,
        /// Prior exact revision.
        from: RevisionId,
        /// New exact revision.
        to: RevisionId,
    },
    /// Scope snapshot changed.
    Snapshot {
        /// Prior scope snapshot.
        from: SnapshotId,
        /// New scope snapshot.
        to: SnapshotId,
    },
    /// Completeness scope identity/revision changed.
    Scope {
        /// Prior opaque scope identity.
        from: String,
        /// New opaque scope identity.
        to: String,
    },
    /// Exact configuration changed.
    Configuration {
        /// Prior exact configuration identity.
        from: String,
        /// New exact configuration identity.
        to: String,
    },
    /// A derived result dependency changed.
    DerivedResult {
        /// Prior immutable result identity.
        from: RevisionId,
        /// New immutable result identity.
        to: RevisionId,
    },
}

/// Reports whether a record explicitly depends on a changed value.
///
/// - `record`: Historical derived result to inspect.
/// - `change`: Exact semantic change.
///
/// `true` only when a declared dependency matches the old changed identity.
#[must_use]
/// # Returns
/// The operation result.
pub fn is_affected(record: &DerivedResultRecord, change: &SemanticChange) -> bool {
    record
        .key()
        .dependencies
        .as_slice()
        .iter()
        .any(|dependency| match (dependency, change) {
            (
                Dependency::ExactRevision { entity, revision },
                SemanticChange::Revision {
                    entity: changed,
                    from,
                    ..
                },
            ) => entity == changed && revision == from,
            (Dependency::Snapshot(snapshot), SemanticChange::Snapshot { from, .. }) => {
                snapshot == from
            }
            (Dependency::Scope(scope), SemanticChange::Scope { from, .. }) => scope == from,
            (Dependency::Configuration(config), SemanticChange::Configuration { from, .. }) => {
                config == from
            }
            (Dependency::DerivedResult(result), SemanticChange::DerivedResult { from, .. }) => {
                result == from
            }
            _ => false,
        })
}

/// Finds direct and transitive dependents of a semantic change.
///
/// A visited-handle set prevents malformed cyclic metadata from looping.
///
/// - `records`: Retained records that may depend on one another.
/// - `change`: Exact semantic change at the graph boundary.
///
/// A deterministic list of affected immutable result handles.
///
/// This traversal is infallible and returns an empty list when nothing matches.
#[must_use]
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn affected_transitively(
    records: &[DerivedResultRecord],
    change: &SemanticChange,
) -> Vec<crate::event::DerivedResultHandle> {
    let mut affected = Vec::new();
    let mut frontier = Vec::new();
    for record in records {
        if is_affected(record, change) {
            let handle = record.handle();
            if !affected.contains(&handle) {
                affected.push(handle);
                frontier.push(handle);
            }
        }
    }
    while let Some(changed_result) = frontier.pop() {
        for record in records {
            let depends_on_changed = record.key().dependencies.as_slice().iter().any(|dependency| {
                matches!(dependency, Dependency::DerivedResult(result) if *result == changed_result.artifact_id)
            });
            let handle = record.handle();
            if depends_on_changed && !affected.contains(&handle) {
                affected.push(handle);
                frontier.push(handle);
            }
        }
    }
    affected.sort();
    affected
}
