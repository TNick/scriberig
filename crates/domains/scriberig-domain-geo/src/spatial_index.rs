//! Builds exact, rebuildable spatial-envelope indexes for feature collections.

use scriberig_core::{EntityId, RevisionId, SnapshotId};

use crate::{
    CoordinateReferenceSystem, Feature, GeoBounds, GeoFeatureHandle, GeoQueryError, GeoQueryPage,
    query::{Decimal, geometry_envelope, validate_bounds},
};

const MAX_SPATIAL_QUERY_PAGE_SIZE: usize = 1_000;

/// Indexes exact feature envelopes derived from one layer revision.
///
/// # Public members
/// - `build`: Builds an index from exact semantic features.
/// - `snapshot`: Returns the source snapshot.
/// - `layer`: Returns the indexed layer.
/// - `source_revision`: Returns the exact source layer revision.
/// - `query`: Selects overlapping feature handles with bounded pagination.
///
/// # Private members
/// - `snapshot`: Exact semantic snapshot used for the index.
/// - `layer`: Stable indexed layer identity.
/// - `source_revision`: Exact layer revision used to derive envelopes.
/// - `entries`: Feature envelopes sorted by minimum x and semantic identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoSpatialIndex {
    /// Exact semantic snapshot used for the index.
    snapshot: SnapshotId,
    /// Stable indexed layer identity.
    layer: EntityId,
    /// Exact layer revision used to derive feature envelopes.
    source_revision: RevisionId,
    /// Feature bounds sorted by minimum x and identity.
    entries: Vec<SpatialEntry>,
}

/// Stores one exact feature envelope in the derived spatial index.
#[derive(Clone, Debug, Eq, PartialEq)]
struct SpatialEntry {
    /// Stable semantic feature identity.
    feature: EntityId,
    /// Minimum x coordinate.
    min_x: Decimal,
    /// Minimum y coordinate.
    min_y: Decimal,
    /// Maximum x coordinate.
    max_x: Decimal,
    /// Maximum y coordinate.
    max_y: Decimal,
}

impl GeoSpatialIndex {
    /// Builds a deterministic spatial index from exact layer features.
    ///
    /// Features without geometry are omitted; empty geometries contribute no entry.
    /// The index is a rebuildable derivation and does not become canonical state.
    ///
    /// # Parameters
    /// - `features`: Semantic features from the exact layer revision.
    /// - `snapshot`: Exact canonical snapshot containing the layer.
    /// - `layer`: Semantic layer identity being indexed.
    /// - `source_revision`: Exact layer revision that owns the feature collection.
    ///
    /// # Returns
    /// A deterministic index of feature envelopes using exact decimal coordinates.
    ///
    /// # Errors
    /// Returns `WrongLayer` for features belonging to a different layer and
    /// `InvalidBounds` for malformed coordinate values.
    pub fn build(
        features: impl IntoIterator<Item = Feature>,
        snapshot: SnapshotId,
        layer: EntityId,
        source_revision: RevisionId,
    ) -> Result<Self, GeoQueryError> {
        let mut entries = Vec::new();
        for feature in features {
            if feature.layer != layer {
                return Err(GeoQueryError::WrongLayer);
            }
            let Some(geometry) = feature.geometry else {
                continue;
            };
            let Some((min_x, min_y, max_x, max_y)) = geometry_envelope(&geometry)? else {
                continue;
            };
            entries.push(SpatialEntry {
                feature: feature.id,
                min_x,
                min_y,
                max_x,
                max_y,
            });
        }
        entries.sort_by(|left, right| {
            left.min_x
                .cmp(&right.min_x)
                .then_with(|| left.feature.cmp(&right.feature))
        });
        Ok(Self {
            snapshot,
            layer,
            source_revision,
            entries,
        })
    }

    /// Returns the exact snapshot used by the index.
    ///
    /// # Returns
    /// Immutable canonical snapshot identity.
    #[must_use]
    pub const fn snapshot(&self) -> SnapshotId {
        self.snapshot
    }

    /// Returns the semantic layer identity indexed by this result.
    ///
    /// # Returns
    /// Stable layer identity.
    #[must_use]
    pub const fn layer(&self) -> EntityId {
        self.layer
    }

    /// Returns the exact layer revision used to build this index.
    ///
    /// # Returns
    /// Immutable source revision identity.
    #[must_use]
    pub const fn source_revision(&self) -> RevisionId {
        self.source_revision
    }

    /// Finds feature handles whose exact envelopes overlap explicit bounds.
    ///
    /// Results are ordered by semantic feature ID, independent of the internal
    /// minimum-x ordering used to prune candidates.
    ///
    /// # Parameters
    /// - `bounds`: Query envelope and explicit coordinate reference system.
    /// - `layer_crs`: Exact CRS declared by the indexed layer.
    /// - `offset`: Number of matching handles skipped.
    /// - `limit`: Requested page size, from one through one thousand.
    ///
    /// # Returns
    /// Snapshot-bound matching feature handles and continuation state.
    ///
    /// # Errors
    /// Returns `InvalidBounds` for malformed/reversed values or `CrsMismatch`
    /// when the explicit query CRS does not equal the indexed layer CRS.
    pub fn query(
        &self,
        bounds: &GeoBounds,
        layer_crs: Option<&CoordinateReferenceSystem>,
        offset: usize,
        limit: usize,
    ) -> Result<GeoQueryPage, GeoQueryError> {
        if limit == 0 || limit > MAX_SPATIAL_QUERY_PAGE_SIZE {
            return Err(GeoQueryError::InvalidPageSize);
        }
        validate_bounds(bounds)?;
        if layer_crs != Some(&bounds.crs) {
            return Err(GeoQueryError::CrsMismatch);
        }
        let min_x = Decimal::parse(&bounds.min_x)?;
        let min_y = Decimal::parse(&bounds.min_y)?;
        let max_x = Decimal::parse(&bounds.max_x)?;
        let max_y = Decimal::parse(&bounds.max_y)?;
        let mut matching = self
            .entries
            .iter()
            .take_while(|entry| entry.min_x <= max_x)
            .filter(|entry| entry.max_x >= min_x && entry.min_y <= max_y && entry.max_y >= min_y)
            .map(|entry| entry.feature)
            .collect::<Vec<_>>();
        matching.sort_unstable();
        let start = offset.min(matching.len());
        let end = start.saturating_add(limit).min(matching.len());
        let matches = matching[start..end]
            .iter()
            .map(|feature| GeoFeatureHandle {
                feature: *feature,
                layer: self.layer,
                snapshot: self.snapshot,
            })
            .collect();
        Ok(GeoQueryPage {
            matches,
            has_more: end < matching.len(),
        })
    }
}

#[cfg(test)]
#[path = "spatial_index/tests.rs"]
mod tests;
