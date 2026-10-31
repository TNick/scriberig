//! Verifies exact, snapshot-bound spatial envelope index behavior.

use super::GeoSpatialIndex;
use crate::{Coordinate, CoordinateReferenceSystem, Feature, GeoBounds, GeoQueryError, Geometry};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use std::collections::BTreeMap;

/// Creates a point feature at exact decimal coordinate text.
///
/// # Parameters
/// - `layer`: Owning semantic layer.
/// - `id`: Stable semantic feature identity.
/// - `x`: Exact x coordinate text.
/// - `y`: Exact y coordinate text.
///
/// # Returns
/// Point feature with empty attributes and provider-native evidence.
fn point(layer: EntityId, id: EntityId, x: &str, y: &str) -> Feature {
    Feature {
        id,
        layer,
        native_id: Some(id.to_string()),
        geometry: Some(Geometry::Point(Coordinate {
            x: x.to_owned(),
            y: y.to_owned(),
            z: None,
            measure: None,
        })),
        attributes: BTreeMap::new(),
    }
}

/// Builds an exact spatial index and returns bounded snapshot-bound handles.
#[test]
fn spatial_index_prunes_envelopes_and_keeps_exact_source_scope() {
    let layer = EntityId::from_bytes([1; 16]);
    let first = EntityId::from_bytes([2; 16]);
    let second = EntityId::from_bytes([3; 16]);
    let snapshot = SnapshotId::new([4; 32]);
    let revision = RevisionId::new([5; 32]);
    let crs = CoordinateReferenceSystem {
        authority: Some("EPSG".to_owned()),
        code: Some("4326".to_owned()),
        definition: None,
    };
    let index = GeoSpatialIndex::build(
        [
            point(layer, first, "9007199254740993.1", "2"),
            point(layer, second, "9007199254740994", "2"),
        ],
        snapshot,
        layer,
        revision,
    )
    .expect("build exact index");
    let page = index
        .query(
            &GeoBounds {
                min_x: "9007199254740993.0".to_owned(),
                min_y: "1".to_owned(),
                max_x: "9007199254740993.2".to_owned(),
                max_y: "3".to_owned(),
                crs: crs.clone(),
            },
            Some(&crs),
            0,
            1,
        )
        .expect("query exact envelope");
    assert_eq!(page.matches.len(), 1);
    assert!(!page.has_more);
    assert_eq!(page.matches[0].feature, first);
    assert_eq!(page.matches[0].snapshot, snapshot);
    assert_eq!(index.layer(), layer);
    assert_eq!(index.snapshot(), snapshot);
    assert_eq!(index.source_revision(), revision);
}

/// Rejects an index query whose explicit CRS differs from its indexed layer.
#[test]
fn spatial_index_requires_matching_crs() {
    let layer = EntityId::new();
    let crs = CoordinateReferenceSystem {
        authority: Some("EPSG".to_owned()),
        code: Some("4326".to_owned()),
        definition: None,
    };
    let index = GeoSpatialIndex::build(
        [point(layer, EntityId::new(), "0", "0")],
        SnapshotId::new([6; 32]),
        layer,
        RevisionId::new([7; 32]),
    )
    .expect("build index");
    let query_crs = CoordinateReferenceSystem {
        code: Some("3857".to_owned()),
        ..crs.clone()
    };
    assert_eq!(
        index.query(
            &GeoBounds {
                min_x: "-1".to_owned(),
                min_y: "-1".to_owned(),
                max_x: "1".to_owned(),
                max_y: "1".to_owned(),
                crs: query_crs,
            },
            Some(&crs),
            0,
            10,
        ),
        Err(GeoQueryError::CrsMismatch)
    );
}
