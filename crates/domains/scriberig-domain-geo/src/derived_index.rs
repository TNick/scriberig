//! Builds exact, rebuildable geospatial attribute indexes with derivation evidence.

use std::collections::{BTreeMap, BTreeSet};

use scriberig_core::{CanonicalWriter, EntityId, KnowledgeValue, RevisionId, SnapshotId};
use scriberig_derive::{
    Dependency, DependencySet, DerivationKey, DerivationProfile, DerivedRepository,
    DerivedResultHandle, DerivedResultRecord, DerivedResultStatus, RepositoryError,
    RetentionPolicy, RetentionStrength,
};

use crate::{Feature, GeoValue};

#[path = "derived_index/decode.rs"]
mod decode;

const MAX_INDEX_QUERY_PAGE_SIZE: usize = 1_000;

/// Stores a deterministic attribute index derived from one exact layer revision.
///
/// # Public members
/// - `build`: Builds the index from exact layer features.
/// - `snapshot`: Returns the exact snapshot used by the index.
/// - `layer`: Returns the indexed layer identity.
/// - `source_revision`: Returns the exact canonical layer revision.
/// - `lookup`: Returns stable feature handles for one exact field/value pair.
/// - `derived_record`: Creates an immutable derivation record for shared storage.
/// - `persist`: Inserts the record into an existing shared derived repository.
///
/// # Private members
/// - `snapshot`: Exact canonical snapshot used by the derivation.
/// - `layer`: Indexed semantic layer.
/// - `source_revision`: Exact immutable layer revision.
/// - `entries`: Deterministically ordered attribute/value lookup table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoAttributeIndex {
    /// Exact canonical snapshot used by the derivation.
    snapshot: SnapshotId,
    /// Indexed semantic layer.
    layer: EntityId,
    /// Exact immutable layer revision.
    source_revision: RevisionId,
    /// Canonical attribute/value key mapped to stable feature IDs.
    entries: BTreeMap<Vec<u8>, Vec<EntityId>>,
}

/// Reports invalid feature membership while building a derived attribute index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeoIndexError {
    /// A feature does not belong to the indexed layer.
    WrongLayer,
    /// The source contains more than one record for a semantic feature identity.
    DuplicateFeature,
    /// A stored derivation record does not match this index format or its dependencies.
    InvalidDerivedRecord,
}

impl GeoAttributeIndex {
    /// Restores a typed index from an exact shared derived-result record.
    ///
    /// The record must have the expected derivation profile, payload digest,
    /// layer revision dependency, and snapshot dependency.
    ///
    /// # Parameters
    /// - `record`: Immutable result loaded from a shared derived repository.
    ///
    /// # Returns
    /// Restored index when the record is exact and successful.
    ///
    /// # Errors
    /// Returns `InvalidDerivedRecord` when record identity or payload is inconsistent.
    pub fn from_derived_record(record: &DerivedResultRecord) -> Result<Self, GeoIndexError> {
        decode::from_derived_record(record)
    }

    /// Builds an attribute index bound to an exact snapshot and source revision.
    ///
    /// # Parameters
    /// - `features`: Semantic features from the exact layer revision.
    /// - `snapshot`: Exact canonical snapshot containing the layer.
    /// - `layer`: Semantic layer identity being indexed.
    /// - `source_revision`: Exact layer revision that owns the complete collection.
    ///
    /// # Returns
    /// A deterministic attribute-to-feature index.
    ///
    /// # Errors
    /// Returns an error when a feature belongs to another layer or is duplicated.
    pub fn build(
        features: impl IntoIterator<Item = Feature>,
        snapshot: SnapshotId,
        layer: EntityId,
        source_revision: RevisionId,
    ) -> Result<Self, GeoIndexError> {
        let mut entries: BTreeMap<Vec<u8>, Vec<EntityId>> = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for feature in features {
            if feature.layer != layer {
                return Err(GeoIndexError::WrongLayer);
            }
            if !seen.insert(feature.id) {
                return Err(GeoIndexError::DuplicateFeature);
            }
            for (field, value) in feature.attributes {
                entries
                    .entry(attribute_key(&field, &value))
                    .or_default()
                    .push(feature.id);
            }
        }
        for feature_ids in entries.values_mut() {
            feature_ids.sort();
        }
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

    /// Returns the exact canonical layer revision used to build the index.
    ///
    /// # Returns
    /// Immutable source revision identity.
    #[must_use]
    pub const fn source_revision(&self) -> RevisionId {
        self.source_revision
    }

    /// Looks up feature identities by exact field and typed value.
    ///
    /// # Parameters
    /// - `field`: Authored schema field name.
    /// - `value`: Exact typed attribute value.
    ///
    /// # Returns
    /// Stable feature identities in deterministic order, or `None` when unmatched.
    #[must_use]
    pub fn lookup(&self, field: &str, value: &GeoValue) -> Option<&[EntityId]> {
        self.entries
            .get(&attribute_key(field, value))
            .map(Vec::as_slice)
    }

    /// Queries the derived attribute index with a bounded snapshot-scoped page.
    ///
    /// # Parameters
    /// - `field`: Authored schema field name.
    /// - `value`: Exact typed attribute value.
    /// - `offset`: Number of matching features skipped.
    /// - `limit`: Requested page size, at most 1000.
    ///
    /// # Returns
    /// Feature handles bound to this index snapshot and continuation state.
    ///
    /// # Errors
    /// Returns `InvalidPageSize` when the requested limit is zero or exceeds 1000.
    pub fn query_attribute(
        &self,
        field: &str,
        value: &GeoValue,
        offset: usize,
        limit: usize,
    ) -> Result<crate::GeoQueryPage, crate::GeoQueryError> {
        if limit == 0 || limit > MAX_INDEX_QUERY_PAGE_SIZE {
            return Err(crate::GeoQueryError::InvalidPageSize);
        }
        let ids = self.lookup(field, value).unwrap_or_default();
        let start = offset.min(ids.len());
        let end = start.saturating_add(limit).min(ids.len());
        let matches = ids[start..end]
            .iter()
            .map(|feature| crate::GeoFeatureHandle {
                feature: *feature,
                layer: self.layer,
                snapshot: self.snapshot,
            })
            .collect();
        Ok(crate::GeoQueryPage {
            matches,
            has_more: end < ids.len(),
        })
    }

    /// Creates a shared derivation record with exact source dependencies.
    ///
    /// # Returns
    /// Immutable reusable derived-result record for the shared derivation repository.
    ///
    /// # Errors
    /// This method is infallible; persistence errors are reported by [`Self::persist`].
    #[must_use]
    pub fn derived_record(&self) -> DerivedResultRecord {
        let payload = self.canonical_bytes();
        let profile = DerivationProfile::new("geo.attribute-index", "1")
            .with_producer("scriberig-domain-geo")
            .with_configuration("typed-exact-attribute-v1");
        let dependencies = DependencySet::new(vec![
            Dependency::ExactRevision {
                entity: self.layer,
                revision: self.source_revision,
            },
            Dependency::Snapshot(self.snapshot),
        ]);
        let key = DerivationKey::new(profile, dependencies);
        let artifact_id = RevisionId::new(*blake3::hash(&payload).as_bytes());
        DerivedResultRecord::new(
            key,
            DerivedResultHandle { artifact_id },
            DerivedResultStatus::Succeeded,
        )
        .with_retention(
            RetentionPolicy::ReusableExpensive,
            RetentionStrength::WeakDependency,
        )
        .with_payload(KnowledgeValue::Bytes(payload))
    }

    /// Persists this index using the repository shared by all domain derivations.
    ///
    /// # Parameters
    /// - `repository`: Shared immutable derived-result repository.
    ///
    /// # Returns
    /// Stable handle for the inserted or already present index result.
    ///
    /// # Errors
    /// Returns a conflict if the result handle already names different content.
    pub fn persist(
        &self,
        repository: &mut impl DerivedRepository,
    ) -> Result<DerivedResultHandle, RepositoryError> {
        let record = self.derived_record();
        let handle = record.handle();
        repository.insert(record)?;
        Ok(handle)
    }

    /// Encodes the derived index and its exact input identity deterministically.
    ///
    /// # Returns
    /// Versioned canonical bytes for immutable result identity and storage.
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut writer = CanonicalWriter::new();
        writer.write_tag(b"scriberig-geo-attribute-index-v1\0");
        writer.write_bytes(self.layer.as_bytes());
        writer.write_bytes(self.source_revision.as_bytes());
        writer.write_bytes(self.snapshot.as_bytes());
        writer.write_u64(self.entries.len() as u64);
        for (key, feature_ids) in &self.entries {
            writer.write_bytes(key);
            writer.write_u64(feature_ids.len() as u64);
            for feature_id in feature_ids {
                writer.write_bytes(feature_id.as_bytes());
            }
        }
        writer.into_bytes()
    }
}

/// Encodes a typed attribute pair into a collision-free deterministic key.
///
/// # Parameters
/// - `field`: Authored attribute name.
/// - `value`: Exact typed attribute value.
///
/// # Returns
/// Canonical bytes distinguishing both field name and value variant.
fn attribute_key(field: &str, value: &GeoValue) -> Vec<u8> {
    let mut writer = CanonicalWriter::new();
    writer.write_tag(b"scriberig-geo-attribute-key-v1\0");
    writer.write_str(field);
    match value {
        GeoValue::Null => writer.write_u8(0),
        GeoValue::Text(value) => {
            writer.write_u8(1);
            writer.write_str(value);
        }
        GeoValue::Number(value) => {
            writer.write_u8(2);
            writer.write_str(value);
        }
        GeoValue::Boolean(value) => {
            writer.write_u8(3);
            writer.write_bool(*value);
        }
        GeoValue::DateTime(value) => {
            writer.write_u8(4);
            writer.write_str(value);
        }
    }
    writer.into_bytes()
}

#[cfg(test)]
#[path = "derived_index/tests.rs"]
mod tests;
