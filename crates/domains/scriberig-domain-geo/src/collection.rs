//! Implements stable, bounded access to semantic geospatial features.

use std::collections::BTreeMap;

use scriberig_core::{EntityId, SnapshotId};
use serde::{Deserialize, Serialize};

use crate::{
    Feature, GeoPredicate, GeoQueryError, GeoQueryPage,
    query::{GeoFeatureHandle, matches, validate_predicate},
};

/// Stores features by stable semantic identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FeatureCollection {
    /// Maximum page size selected by the collection owner.
    page_limit: usize,
    /// Sparse semantic feature records keyed by exact entity identity.
    features: BTreeMap<EntityId, Feature>,
}

/// Represents one bounded feature page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeaturePage {
    /// Features in stable semantic identity order.
    pub features: Vec<Feature>,
    /// Whether more features follow this page.
    pub has_more: bool,
}

impl FeatureCollection {
    /// Creates a collection from unique semantic feature identities.
    ///
    /// - `features`: Imported feature records.
    /// - `page_limit`: Maximum page size, between one and 1000.
    ///
    /// # Returns
    /// Validated feature collection.
    ///
    /// Returns an error for an invalid page limit or duplicate identity.
    ///
    /// # Errors
    /// Returns an error when the page limit is invalid or a feature identity is duplicated.
    pub fn new(
        features: impl IntoIterator<Item = Feature>,
        page_limit: usize,
    ) -> Result<Self, CollectionError> {
        if !(1..=1000).contains(&page_limit) {
            return Err(CollectionError::InvalidPageLimit);
        }
        let mut indexed = BTreeMap::new();
        for feature in features {
            if indexed.insert(feature.id, feature).is_some() {
                return Err(CollectionError::DuplicateFeature);
            }
        }
        Ok(Self {
            page_limit,
            features: indexed,
        })
    }

    /// Reads a bounded page in stable identity order.
    ///
    /// - `offset`: Number of features skipped before reading.
    /// - `limit`: Requested page size, no greater than the configured maximum.
    ///
    /// # Returns
    /// Feature page and continuation state.
    ///
    /// Returns an error when the requested page size is invalid.
    ///
    /// # Errors
    /// Returns `InvalidPageLimit` when the requested page size exceeds the configured limit.
    pub fn page(&self, offset: usize, limit: usize) -> Result<FeaturePage, CollectionError> {
        if limit == 0 || limit > self.page_limit {
            return Err(CollectionError::InvalidPageLimit);
        }
        let selected = self
            .features
            .values()
            .skip(offset)
            .take(limit + 1)
            .collect::<Vec<_>>();
        let has_more = selected.len() > limit;
        Ok(FeaturePage {
            features: selected.into_iter().take(limit).cloned().collect(),
            has_more,
        })
    }

    /// Queries features by exact attribute value or CRS-bound geometry envelope.
    ///
    /// - `predicate`: Predicate evaluated over the current collection contents.
    /// - `snapshot`: Exact canonical snapshot that owns these feature contents.
    /// - `layer_crs`: Layer CRS required for geometry-envelope predicates.
    /// - `offset`: Number of matching features skipped.
    /// - `limit`: Requested result size, bounded by the configured page maximum.
    ///
    /// # Returns
    /// Matching handles tied to the requested snapshot and continuation state.
    ///
    /// Returns an error for invalid page sizes, malformed bounds, or CRS mismatch.
    ///
    /// # Errors
    /// Returns an error for invalid page sizes, malformed bounds, or CRS mismatch.
    pub fn query(
        &self,
        predicate: &GeoPredicate,
        snapshot: SnapshotId,
        layer_crs: Option<&crate::CoordinateReferenceSystem>,
        offset: usize,
        limit: usize,
    ) -> Result<GeoQueryPage, GeoQueryError> {
        if limit == 0 || limit > self.page_limit {
            return Err(GeoQueryError::InvalidPageSize);
        }
        validate_predicate(predicate, layer_crs)?;
        let mut handles = Vec::with_capacity(limit + 1);
        let mut matched_before = 0;
        for feature in self.features.values() {
            if !matches(feature, predicate, layer_crs)? {
                continue;
            }
            if matched_before < offset {
                matched_before += 1;
                continue;
            }
            handles.push(GeoFeatureHandle {
                feature: feature.id,
                layer: feature.layer,
                snapshot,
            });
            if handles.len() > limit {
                break;
            }
        }
        let has_more = handles.len() > limit;
        handles.truncate(limit);
        Ok(GeoQueryPage {
            matches: handles,
            has_more,
        })
    }

    /// Replaces one feature while keeping all unrelated identities unchanged.
    ///
    /// - `feature`: Exact semantic replacement feature.
    ///
    /// # Returns
    /// The stable identity of the changed feature.
    pub fn set(&mut self, feature: Feature) -> EntityId {
        let id = feature.id;
        self.features.insert(id, feature);
        id
    }
}

/// Describes invalid page requests or duplicate semantic feature identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollectionError {
    /// A requested/configured page size is outside the allowed range.
    InvalidPageLimit,
    /// Two source features resolved to the same semantic identity.
    DuplicateFeature,
}
