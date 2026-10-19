//! Runs bounded remote entity discovery for the TUI.

use super::{ApplicationError, remote_error};

/// Finds remote entities in an exact snapshot.
///
/// - `client`: Remote protocol client.
/// - `snapshot`: Exact discovery scope.
/// - `query`: Search text.
///
/// A bounded TUI discovery model.
///
/// Returns a sanitized remote application error.
pub(crate) fn find_entities<T: scriberig_protocol::ProtocolTransport>(
    client: &scriberig_client::Client<T>,
    snapshot: scriberig_core::SnapshotId,
    query: &str,
) -> Result<crate::view::find::FindModel, ApplicationError> {
    let result = client
        .find(scriberig_query::FindRequest {
            scope: scriberig_query::QueryScope::Snapshot(snapshot),
            query: query.to_owned(),
            strategy: scriberig_query::FindStrategy::ExactText,
            page: scriberig_query::PageRequest::default(),
            detail: scriberig_query::DetailLevel::Compact,
        })
        .map_err(remote_error)?;
    Ok(crate::view::find::FindModel::from_result(query, &result))
}
