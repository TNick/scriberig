//! Projects bounded semantic discovery results into terminal rows.

use scriberig_core::{EntityId, SnapshotId};

/// One stable find result row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindRow {
    /// Entity reached by the discovery result.
    pub entity: EntityId,
    /// Exact snapshot containing the result.
    pub snapshot: SnapshotId,
    /// Optional type summary.
    pub entity_type: Option<String>,
    /// Optional bounded preview.
    pub preview: Option<String>,
}

/// Bounded discovery model shown by the primary view.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FindModel {
    /// Query text used to produce the rows.
    pub query: String,
    /// Stable result rows.
    pub rows: Vec<FindRow>,
}

impl FindModel {
    /// Builds a terminal model from a public find result.
    ///
    /// # Parameters
    /// - `query`: User-authored discovery text.
    /// - `result`: Application-owned discovery result.
    ///
    /// # Returns
    /// A bounded presentation model.
    #[must_use]
    pub fn from_result(query: impl Into<String>, result: &scriberig_query::FindResult) -> Self {
        Self {
            query: query.into(),
            rows: result
                .page
                .items
                .iter()
                .filter_map(|hit| match hit.target {
                    scriberig_query::TargetRef::Snapshot(target) => Some(FindRow {
                        entity: target.entity,
                        snapshot: target.snapshot,
                        entity_type: hit.entity_type.as_ref().map(ToString::to_string),
                        preview: hit.preview.clone().or_else(|| hit.label.clone()),
                    }),
                    scriberig_query::TargetRef::Revision(_) => None,
                })
                .collect(),
        }
    }
}
