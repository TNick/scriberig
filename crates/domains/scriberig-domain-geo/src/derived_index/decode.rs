//! Restores canonical geospatial attribute index payloads from shared derivations.

use std::collections::BTreeMap;

use scriberig_core::{EntityId, KnowledgeValue, RevisionId, SnapshotId};
use scriberig_derive::{Dependency, DependencySet, DerivedResultRecord, DerivedResultStatus};

use super::{GeoAttributeIndex, GeoIndexError};

const INDEX_TAG: &[u8] = b"scriberig-geo-attribute-index-v1\0";

/// Restores and verifies one immutable geospatial index record.
///
/// # Parameters
/// - `record`: Shared derived result to validate and decode.
///
/// # Returns
/// Typed attribute index with exact source dependencies.
///
/// # Errors
/// Returns `InvalidDerivedRecord` for a wrong profile, status, digest, dependency, or payload.
pub(super) fn from_derived_record(
    record: &DerivedResultRecord,
) -> Result<GeoAttributeIndex, GeoIndexError> {
    if record.status() != DerivedResultStatus::Succeeded
        || record.key().profile.kind() != "geo.attribute-index"
        || record.key().profile.version() != "1"
        || record.key().profile.producer() != Some("scriberig-domain-geo")
        || record.key().profile.configuration() != Some("typed-exact-attribute-v1")
    {
        return Err(GeoIndexError::InvalidDerivedRecord);
    }
    let Some(KnowledgeValue::Bytes(payload)) = record.payload() else {
        return Err(GeoIndexError::InvalidDerivedRecord);
    };
    if *blake3::hash(payload).as_bytes() != *record.handle().artifact_id.as_bytes() {
        return Err(GeoIndexError::InvalidDerivedRecord);
    }
    let mut reader = CanonicalReader::new(payload);
    if reader.read_tag(INDEX_TAG).is_err() {
        return Err(GeoIndexError::InvalidDerivedRecord);
    }
    let layer = read_entity(&mut reader)?;
    let source_revision = read_revision(&mut reader)?;
    let snapshot = read_snapshot(&mut reader)?;
    let expected_dependencies = DependencySet::new(vec![
        Dependency::ExactRevision {
            entity: layer,
            revision: source_revision,
        },
        Dependency::Snapshot(snapshot),
    ]);
    if record.dependencies() != &expected_dependencies {
        return Err(GeoIndexError::InvalidDerivedRecord);
    }
    let entry_count = reader.read_count()?;
    let mut entries = BTreeMap::new();
    let mut previous_key = None;
    for _ in 0..entry_count {
        let key = reader.read_bytes()?.to_vec();
        if previous_key
            .as_ref()
            .is_some_and(|previous| previous >= &key)
        {
            return Err(GeoIndexError::InvalidDerivedRecord);
        }
        previous_key = Some(key.clone());
        let feature_count = reader.read_count()?;
        let mut features = Vec::with_capacity(feature_count);
        let mut previous_feature = None;
        for _ in 0..feature_count {
            let feature = read_entity(&mut reader)?;
            if previous_feature.is_some_and(|previous| previous >= feature) {
                return Err(GeoIndexError::InvalidDerivedRecord);
            }
            previous_feature = Some(feature);
            features.push(feature);
        }
        if features.is_empty() || entries.insert(key, features).is_some() {
            return Err(GeoIndexError::InvalidDerivedRecord);
        }
    }
    if !reader.is_empty() {
        return Err(GeoIndexError::InvalidDerivedRecord);
    }
    Ok(GeoAttributeIndex {
        snapshot,
        layer,
        source_revision,
        entries,
    })
}

/// Reads one fixed-width entity ID from the canonical payload.
///
/// # Parameters
/// - `reader`: Payload reader positioned at a length-prefixed ID.
///
/// # Returns
/// Stable semantic entity identity.
///
/// # Errors
/// Returns `InvalidDerivedRecord` when the ID is malformed or truncated.
fn read_entity(reader: &mut CanonicalReader<'_>) -> Result<EntityId, GeoIndexError> {
    reader
        .read_bytes()?
        .try_into()
        .map(EntityId::from_bytes)
        .map_err(|_| GeoIndexError::InvalidDerivedRecord)
}

/// Reads one fixed-width revision ID from the canonical payload.
///
/// # Parameters
/// - `reader`: Payload reader positioned at a length-prefixed ID.
///
/// # Returns
/// Exact immutable revision identity.
///
/// # Errors
/// Returns `InvalidDerivedRecord` when the ID is malformed or truncated.
fn read_revision(reader: &mut CanonicalReader<'_>) -> Result<RevisionId, GeoIndexError> {
    reader
        .read_bytes()?
        .try_into()
        .map(RevisionId::new)
        .map_err(|_| GeoIndexError::InvalidDerivedRecord)
}

/// Reads one fixed-width snapshot ID from the canonical payload.
///
/// # Parameters
/// - `reader`: Payload reader positioned at a length-prefixed ID.
///
/// # Returns
/// Exact immutable snapshot identity.
///
/// # Errors
/// Returns `InvalidDerivedRecord` when the ID is malformed or truncated.
fn read_snapshot(reader: &mut CanonicalReader<'_>) -> Result<SnapshotId, GeoIndexError> {
    reader
        .read_bytes()?
        .try_into()
        .map(SnapshotId::new)
        .map_err(|_| GeoIndexError::InvalidDerivedRecord)
}

/// Reads length-delimited canonical values with bounds checking.
struct CanonicalReader<'a> {
    /// Exact immutable payload bytes.
    bytes: &'a [u8],
    /// Next unread payload offset.
    offset: usize,
}

impl<'a> CanonicalReader<'a> {
    /// Creates a reader at the start of an exact payload.
    ///
    /// # Parameters
    /// - `bytes`: Exact canonical payload.
    ///
    /// # Returns
    /// Reader positioned at byte zero.
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    /// Checks and consumes one fixed version tag.
    ///
    /// # Parameters
    /// - `self`: Payload reader.
    /// - `expected`: Exact supported payload tag.
    ///
    /// # Returns
    /// Unit when the next bytes match.
    ///
    /// # Errors
    /// Returns `InvalidDerivedRecord` when the payload is truncated or has another tag.
    fn read_tag(&mut self, expected: &[u8]) -> Result<(), GeoIndexError> {
        if self.read_exact(expected.len())? != expected {
            return Err(GeoIndexError::InvalidDerivedRecord);
        }
        Ok(())
    }

    /// Reads one length-prefixed byte string.
    ///
    /// # Parameters
    /// - `self`: Payload reader.
    ///
    /// # Returns
    /// Borrowed byte string from the payload.
    ///
    /// # Errors
    /// Returns `InvalidDerivedRecord` for invalid lengths or truncated bytes.
    fn read_bytes(&mut self) -> Result<&'a [u8], GeoIndexError> {
        let length =
            usize::try_from(self.read_u64()?).map_err(|_| GeoIndexError::InvalidDerivedRecord)?;
        self.read_exact(length)
    }

    /// Reads a bounded sequence length from the payload.
    ///
    /// # Parameters
    /// - `self`: Payload reader.
    ///
    /// # Returns
    /// Sequence element count that cannot exceed remaining payload bytes.
    ///
    /// # Errors
    /// Returns `InvalidDerivedRecord` when the count cannot fit or is implausible.
    fn read_count(&mut self) -> Result<usize, GeoIndexError> {
        let count =
            usize::try_from(self.read_u64()?).map_err(|_| GeoIndexError::InvalidDerivedRecord)?;
        (count <= self.bytes.len().saturating_sub(self.offset))
            .then_some(count)
            .ok_or(GeoIndexError::InvalidDerivedRecord)
    }

    /// Reads one big-endian unsigned length.
    ///
    /// # Parameters
    /// - `self`: Payload reader.
    ///
    /// # Returns
    /// Decoded 64-bit length.
    ///
    /// # Errors
    /// Returns `InvalidDerivedRecord` when fewer than eight bytes remain.
    fn read_u64(&mut self) -> Result<u64, GeoIndexError> {
        self.read_exact(8)?
            .try_into()
            .map(u64::from_be_bytes)
            .map_err(|_| GeoIndexError::InvalidDerivedRecord)
    }

    /// Borrows a fixed byte range and advances the reader.
    ///
    /// # Parameters
    /// - `self`: Payload reader.
    /// - `length`: Number of bytes to consume.
    ///
    /// # Returns
    /// Borrowed exact payload slice.
    ///
    /// # Errors
    /// Returns `InvalidDerivedRecord` when the requested bytes exceed the payload.
    fn read_exact(&mut self, length: usize) -> Result<&'a [u8], GeoIndexError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(GeoIndexError::InvalidDerivedRecord)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(GeoIndexError::InvalidDerivedRecord)?;
        self.offset = end;
        Ok(value)
    }

    /// Reports whether the complete payload has been consumed.
    ///
    /// # Returns
    /// True when no trailing bytes remain.
    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }
}
