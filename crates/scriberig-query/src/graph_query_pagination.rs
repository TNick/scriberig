//! Computes bounded query pagination offsets.

/// Computes bounded page offsets from a provider-owned continuation token.
/// - `super`: Parent-module imports used by this helper.
///
/// # Parameters
/// - `continuation`: Optional provider continuation token.
/// - `limit`: Maximum number of items to return.
/// - `operation`: Operation name encoded by the token.
/// - `snapshot`: Snapshot encoded by the token.
///
/// # Returns
/// Start and exclusive end offsets.
///
/// # Errors
/// Returns an invalid-continuation error when the token is malformed.
pub(super) fn page_bounds(
    continuation: Option<&crate::ContinuationToken>,
    limit: usize,
    operation: &str,
    snapshot: scriberig_core::SnapshotId,
) -> Result<(usize, usize), crate::QueryError> {
    if !(1..=crate::MAX_PAGE_LIMIT).contains(&limit) {
        return Err(crate::QueryError::InvalidQuery {
            message: format!("page limit must be between 1 and {}", crate::MAX_PAGE_LIMIT),
        });
    }
    let start = continuation.map_or(0, |token| {
        token.offset_for(operation, snapshot).unwrap_or(usize::MAX)
    });
    if start == usize::MAX {
        return Err(crate::QueryError::InvalidContinuation {
            operation: crate::ReadOperation::Query,
        });
    }
    Ok((start, start.saturating_add(limit)))
}
