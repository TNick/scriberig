//! Registers geospatial attribute selection with the shared typed query API.

use scriberig_core::SnapshotId;
use scriberig_query::{DomainQueryExtension, QueryError};
use serde::{Deserialize, Serialize};

use crate::{GeoAttributeIndex, GeoQueryPage, GeoValue};

/// Requests one bounded exact attribute match from a geospatial index.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct GeoAttributeQueryRequest {
    /// Authored layer field to compare.
    pub field: String,
    /// Exact typed attribute value to match.
    pub value: GeoValue,
    /// Number of matching features skipped before this page.
    pub offset: usize,
    /// Maximum number of feature handles returned.
    pub limit: usize,
}

/// Executes the shared typed geospatial attribute query against one index.
///
/// # Public members
/// - `new`: Binds a derived index to the selector.
/// - `index`: Returns the exact index selected for execution.
///
/// # Private members
/// - `index`: Rebuildable derived attribute index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoAttributeSelector {
    /// Rebuildable index bound to an exact layer revision and snapshot.
    index: GeoAttributeIndex,
}

impl GeoAttributeSelector {
    /// Creates a selector over one exact derived attribute index.
    ///
    /// # Parameters
    /// - `index`: Rebuildable index selected for the query.
    ///
    /// # Returns
    /// A typed extension ready to register with `DomainQueryRegistry`.
    #[must_use]
    pub const fn new(index: GeoAttributeIndex) -> Self {
        Self { index }
    }

    /// Returns the exact index used by this selector.
    ///
    /// # Returns
    /// Borrowed snapshot- and revision-bound index.
    #[must_use]
    pub const fn index(&self) -> &GeoAttributeIndex {
        &self.index
    }
}

impl DomainQueryExtension for GeoAttributeSelector {
    type Request = GeoAttributeQueryRequest;
    type Response = GeoQueryPage;

    const ID: &'static str = "geo.features_by_attribute";

    /// Executes one bounded exact-value query against the selected snapshot.
    ///
    /// # Parameters
    /// - `self`: Selector owning the exact derived index.
    /// - `snapshot`: Snapshot requested by the shared query caller.
    /// - `request`: Typed attribute and page request.
    ///
    /// # Returns
    /// Bounded feature handles tied to the same exact snapshot.
    ///
    /// # Errors
    /// Returns an error for a snapshot mismatch or invalid page size.
    fn execute(
        &self,
        snapshot: SnapshotId,
        request: Self::Request,
    ) -> Result<Self::Response, QueryError> {
        if snapshot != self.index.snapshot() {
            return Err(QueryError::InvalidQuery {
                message: "geospatial index does not match the requested snapshot".to_owned(),
            });
        }
        self.index
            .query_attribute(
                &request.field,
                &request.value,
                request.offset,
                request.limit,
            )
            .map_err(|error| QueryError::InvalidQuery {
                message: format!("invalid geospatial attribute query: {error:?}"),
            })
    }
}

#[cfg(test)]
mod tests {
    //! Verifies geospatial queries register through the shared typed query registry.

    use scriberig_core::{EntityId, RevisionId, SnapshotId};
    use scriberig_query::{DomainQueryRegistry, QueryError};

    use super::{GeoAttributeQueryRequest, GeoAttributeSelector};
    use crate::{Feature, GeoAttributeIndex, GeoValue};

    /// Builds one feature with the exact query attribute used by the tests.
    ///
    /// # Parameters
    /// - `id`: Stable semantic feature identity.
    /// - `layer`: Owning layer identity.
    ///
    /// # Returns
    /// Feature with a typed `parcel_id` attribute.
    fn feature(id: EntityId, layer: EntityId) -> Feature {
        Feature {
            id,
            layer,
            native_id: None,
            geometry: None,
            attributes: [("parcel_id".to_owned(), GeoValue::Number("124".to_owned()))]
                .into_iter()
                .collect(),
        }
    }

    /// Checks the shared registry returns bounded spatially scoped feature handles.
    #[test]
    fn shared_registry_executes_snapshot_bound_geo_attribute_query() {
        let snapshot = SnapshotId::new([1; 32]);
        let layer = EntityId::from_bytes([2; 16]);
        let first = EntityId::from_bytes([3; 16]);
        let index = GeoAttributeIndex::build(
            [
                feature(first, layer),
                feature(EntityId::from_bytes([4; 16]), layer),
            ],
            snapshot,
            layer,
            RevisionId::new([5; 32]),
        )
        .expect("build derived index");
        let mut registry = DomainQueryRegistry::new();
        registry
            .register(GeoAttributeSelector::new(index))
            .expect("register geospatial query");
        let result = registry
            .query::<GeoAttributeSelector>(
                snapshot,
                &GeoAttributeQueryRequest {
                    field: "parcel_id".to_owned(),
                    value: GeoValue::Number("124".to_owned()),
                    offset: 0,
                    limit: 1,
                },
            )
            .expect("query exact snapshot");
        assert_eq!(result.snapshot, snapshot);
        assert_eq!(result.response.matches.len(), 1);
        assert!(result.response.has_more);
        assert_eq!(result.response.matches[0].feature, first);
        assert_eq!(result.response.matches[0].snapshot, snapshot);
    }

    /// Rejects querying an index under a different canonical snapshot.
    #[test]
    fn shared_registry_rejects_wrong_geo_index_snapshot() {
        let snapshot = SnapshotId::new([6; 32]);
        let layer = EntityId::new();
        let index = GeoAttributeIndex::build(
            [feature(EntityId::new(), layer)],
            snapshot,
            layer,
            RevisionId::new([7; 32]),
        )
        .expect("build derived index");
        let mut registry = DomainQueryRegistry::new();
        registry
            .register(GeoAttributeSelector::new(index))
            .expect("register geospatial query");
        let result = registry.query::<GeoAttributeSelector>(
            SnapshotId::new([8; 32]),
            &GeoAttributeQueryRequest {
                field: "parcel_id".to_owned(),
                value: GeoValue::Number("124".to_owned()),
                offset: 0,
                limit: 10,
            },
        );
        assert!(matches!(result, Err(QueryError::InvalidQuery { .. })));
    }
}
