//! Verifies typed non-text anchors retain snapshot-bound semantic selections.

use scriberig_core::{EntityId, SnapshotId};
use scriberig_query::{ProjectionAnchor, Selection};

/// Round-trips cell, range, and CRS-aware geometry anchor descriptors.
#[test]
fn typed_anchors_round_trip_without_replacing_semantic_identity() {
    let entity = EntityId::new();
    let worksheet = EntityId::new();
    let snapshot = SnapshotId::new([7; 32]);
    let anchors = [
        ProjectionAnchor::SpreadsheetCell {
            worksheet,
            row: 3,
            column: 2,
        },
        ProjectionAnchor::SpreadsheetRange {
            worksheet,
            first_row: 1,
            first_column: 1,
            last_row: 20,
            last_column: 5,
        },
        ProjectionAnchor::GeometryBounds {
            crs: "EPSG:4326".into(),
            min_x: "-122.4".into(),
            min_y: "37.7".into(),
            max_x: "-122.3".into(),
            max_y: "37.8".into(),
        },
    ];
    for anchor in anchors {
        let selection = Selection {
            snapshot,
            entity,
            revision: None,
            relation_path: Vec::new(),
            projection_anchor: Some(anchor),
        };
        let encoded = serde_json::to_vec(&selection).expect("encode selection");
        let decoded: Selection = serde_json::from_slice(&encoded).expect("decode selection");
        assert_eq!(decoded.entity, entity);
        assert_eq!(decoded.snapshot, snapshot);
        assert_eq!(decoded, selection);
    }
}
