//! Verifies exact derivation dependencies for rebuilt geospatial indexes.

use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_derive::{Dependency, DerivedRepository, InMemoryDerivedRepository};

use super::{GeoAttributeIndex, GeoIndexError};
use crate::{Feature, GeoValue};

/// Creates a minimal indexed feature with one typed attribute.
///
/// # Parameters
/// - `id`: Stable feature identity.
/// - `layer`: Owning layer identity.
/// - `value`: Exact area attribute value.
///
/// # Returns
/// Feature owned by the requested layer.
fn feature(id: EntityId, layer: EntityId, value: &str) -> Feature {
    Feature {
        id,
        layer,
        native_id: None,
        geometry: None,
        attributes: [("area".to_owned(), GeoValue::Number(value.to_owned()))]
            .into_iter()
            .collect(),
    }
}

/// Records an index with the shared derivation repository and checks provenance.
#[test]
fn attribute_index_record_tracks_exact_layer_revision_and_snapshot() {
    let layer = EntityId::from_bytes([1; 16]);
    let first = EntityId::from_bytes([2; 16]);
    let second = EntityId::from_bytes([3; 16]);
    let snapshot = SnapshotId::new([4; 32]);
    let revision = RevisionId::new([5; 32]);
    let index = GeoAttributeIndex::build(
        [feature(second, layer, "12"), feature(first, layer, "12")],
        snapshot,
        layer,
        revision,
    )
    .expect("build exact layer index");
    assert_eq!(
        index.lookup("area", &GeoValue::Number("12".to_owned())),
        Some([first, second].as_slice())
    );
    let encoded = index.derived_record();
    assert_eq!(
        GeoAttributeIndex::from_derived_record(&encoded).expect("restore exact index"),
        index
    );
    let mut repository = InMemoryDerivedRepository::new();
    let handle = index.persist(&mut repository).expect("persist derivation");
    let record = repository.get(handle).expect("stored derived record");
    assert!(
        record
            .dependencies()
            .as_slice()
            .contains(&Dependency::Snapshot(snapshot))
    );
    assert!(
        record
            .dependencies()
            .as_slice()
            .contains(&Dependency::ExactRevision {
                entity: layer,
                revision,
            })
    );
    assert_eq!(record.provenance().inputs.len(), 1);
}

/// Rejects an index that mixes feature revisions from another layer.
#[test]
fn attribute_index_rejects_cross_layer_features() {
    let result = GeoAttributeIndex::build(
        [feature(EntityId::new(), EntityId::new(), "12")],
        SnapshotId::new([6; 32]),
        EntityId::new(),
        RevisionId::new([7; 32]),
    );
    assert_eq!(result, Err(GeoIndexError::WrongLayer));
}
