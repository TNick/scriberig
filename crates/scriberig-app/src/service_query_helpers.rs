//! Converts query-layer inspection values to application results.

use scriberig_query::{DetailLevel as QueryDetailLevel, Inspection, TargetRef};

use crate::{ApplicationError, DetailLevel, InspectRequest, InspectResult};

/// Converts a query inspection into the application result shape.
///
/// - `super`: Application helper module.
/// - `inspection`: Query-layer inspection result.
/// - `_detail`: Detail requested by the caller.
///
/// Snapshot-aware application inspection metadata.
///
/// Returns a revision mismatch when the query target is inconsistent.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn inspect_result(
    inspection: Inspection,
    _detail: DetailLevel,
) -> Result<InspectResult, ApplicationError> {
    let entity = match inspection.subject {
        TargetRef::Snapshot(target) => target.entity,
        TargetRef::Revision(target) => target.entity,
    };
    let revision = match inspection.subject {
        TargetRef::Revision(target) => {
            if inspection
                .revision
                .is_some_and(|value| value != target.revision)
            {
                return Err(ApplicationError::RevisionMismatch {
                    entity,
                    revision: target.revision,
                });
            }
            Some(target.revision)
        }
        TargetRef::Snapshot(_) => inspection.revision,
    };
    Ok(InspectResult {
        entity,
        revision,
        snapshot: inspection.snapshot,
        object_type: inspection.entity_type,
        label: inspection.label,
        description: inspection.description,
        capabilities: inspection
            .operations
            .into_iter()
            .map(|capability| capability.id)
            .chain(inspection.views.into_iter().map(|capability| capability.id))
            .collect(),
        trust: None,
    })
}

/// Converts an application inspection request into an exact query target.
///
/// - `super`: Application helper module.
/// - `request`: Application inspection request.
/// - `current`: Resolved current snapshot for a current request.
///
/// Query-layer inspection request with an exact target.
///
/// Returns an internal error when current snapshot resolution is missing.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn query_request(
    request: &InspectRequest,
    current: Option<scriberig_core::SnapshotId>,
) -> Result<scriberig_query::InspectRequest, ApplicationError> {
    let target = match request.snapshot {
        crate::SnapshotResolution::At(snapshot) => {
            TargetRef::Snapshot(scriberig_query::EntityInSnapshot {
                entity: request.entity,
                snapshot,
            })
        }
        crate::SnapshotResolution::Revision(revision) => {
            TargetRef::Revision(scriberig_query::EntityAtRevision {
                entity: request.entity,
                revision,
            })
        }
        crate::SnapshotResolution::Current => {
            TargetRef::Snapshot(scriberig_query::EntityInSnapshot {
                entity: request.entity,
                snapshot: current.ok_or_else(|| ApplicationError::Internal {
                    message: "current snapshot was not resolved".to_owned(),
                })?,
            })
        }
    };
    Ok(scriberig_query::InspectRequest {
        target,
        detail: match request.detail {
            DetailLevel::Compact => QueryDetailLevel::Compact,
            DetailLevel::Detailed => QueryDetailLevel::Structure,
        },
        view: None,
    })
}
