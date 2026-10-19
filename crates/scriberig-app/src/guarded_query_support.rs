//! Provides private conversions shared by guarded application read methods.

use crate::ApplicationError;
use crate::{ApplicationContext, CapabilityAction, CapabilityScope, OperationKind};
use scriberig_core::EntityId;
use scriberig_query::{FindResult, NavigationEntry, QueryError, QueryResult, TargetRef};

/// Returns the entity represented by a semantic target.
///
/// - `target`: Semantic target to inspect.
/// - `crate`: Owning application crate.
///
/// The stable entity identity represented by the target.
/// # Returns
/// The operation result.
pub(crate) fn target_entity(target: TargetRef) -> EntityId {
    match target {
        TargetRef::Revision(value) => value.entity,
        TargetRef::Snapshot(value) => value.entity,
    }
}

/// Returns the exact snapshot represented by a semantic target.
///
/// - `target`: Semantic target to inspect.
/// - `crate`: Owning application crate.
///
/// The target snapshot, or `None` for an exact revision target.
/// # Returns
/// The operation result.
pub(crate) fn target_snapshot(target: TargetRef) -> Option<scriberig_core::SnapshotId> {
    match target {
        TargetRef::Revision(_) => None,
        TargetRef::Snapshot(value) => Some(value.snapshot),
    }
}

/// Maps reusable query failures into safe application categories.
///
/// - `error`: Provider error to translate.
/// - `crate`: Owning application crate.
///
/// A backend-neutral application error without restricted content.
/// # Returns
/// The operation result.
pub(crate) fn query_error_to_application(error: QueryError) -> ApplicationError {
    match error {
        QueryError::NotFound { target, .. } | QueryError::InvalidTarget { target, .. } => {
            ApplicationError::NotFound {
                subject: format!("{target:?}"),
            }
        }
        QueryError::SnapshotUnavailable { snapshot, .. } => ApplicationError::NotFound {
            subject: format!("{snapshot}"),
        },
        QueryError::UnsupportedCapability { capability, .. } => {
            ApplicationError::UnsupportedOperation {
                operation: capability,
            }
        }
        QueryError::DerivedDataUnavailable { .. } => ApplicationError::UnsupportedOperation {
            operation: "derived_data".to_owned(),
        },
        QueryError::InvalidQuery { message } => ApplicationError::SchemaViolation { message },
        _ => ApplicationError::Internal {
            message: "query provider failed".to_owned(),
        },
    }
}

/// Reports whether a result target is authorized for exposure.
///
/// - `context`: Caller grants to evaluate.
/// - `target`: Semantic target at the result boundary.
/// - `crate`: Owning application crate.
///
/// `true` when the target can be exposed.
/// # Returns
/// The operation result.
pub(crate) fn can_expose(context: &ApplicationContext, target: TargetRef) -> bool {
    let (entity, snapshot) = match target {
        TargetRef::Revision(value) => (value.entity, context.snapshot),
        TargetRef::Snapshot(value) => (value.entity, Some(value.snapshot)),
    };
    context.capabilities.permits(
        &CapabilityAction::Query,
        &CapabilityScope::Entity(entity),
        snapshot,
        Some(OperationKind::Query),
    ) || context.capabilities.permits(
        &CapabilityAction::Inspect,
        &CapabilityScope::Entity(entity),
        snapshot,
        Some(OperationKind::Inspect),
    )
}

/// Removes unauthorized navigation entries in place.
///
/// - `context`: Caller grants to evaluate.
/// - `entries`: Navigation entries to filter.
/// - `crate`: Owning application crate.
pub(crate) fn filter_navigation(context: &ApplicationContext, entries: &mut Vec<NavigationEntry>) {
    entries.retain(|entry| can_expose(context, entry.target));
}

/// Removes unauthorized exact-query rows in place.
///
/// - `context`: Caller grants to evaluate.
/// - `result`: Query result whose rows are filtered.
/// - `crate`: Owning application crate.
pub(crate) fn filter_query_rows(context: &ApplicationContext, result: &mut QueryResult) {
    result
        .page
        .items
        .retain(|row| can_expose(context, row.target));
}

/// Removes unauthorized discovery hits in place.
///
/// - `context`: Caller grants to evaluate.
/// - `result`: Find result whose hits are filtered.
/// - `crate`: Owning application crate.
pub(crate) fn filter_find_hits(context: &ApplicationContext, result: &mut FindResult) {
    result
        .page
        .items
        .retain(|hit| can_expose(context, hit.target));
}
