//! Verifies typed projection anchor transport bounds and wire round trips.

use scriberig_protocol::ProjectionAnchor;

/// Round-trips bounded cell, range, geometry, and adapter anchor descriptors.
#[test]
fn typed_anchor_descriptors_round_trip_with_bounded_fields() {
    let anchors = [
        ProjectionAnchor::SpreadsheetCell {
            worksheet_id: "worksheet-1".to_owned(),
            row: 3,
            column: 2,
        },
        ProjectionAnchor::SpreadsheetRange {
            worksheet_id: "worksheet-1".to_owned(),
            first_row: 1,
            first_column: 1,
            last_row: 20,
            last_column: 5,
        },
        ProjectionAnchor::GeometryBounds {
            crs: "EPSG:4326".to_owned(),
            min_x: "-122.4".to_owned(),
            min_y: "37.7".to_owned(),
            max_x: "-122.3".to_owned(),
            max_y: "37.8".to_owned(),
        },
        ProjectionAnchor::Opaque {
            adapter: "example".to_owned(),
            anchor_kind: "shape".to_owned(),
            value: "shape-1".to_owned(),
        },
    ];
    for anchor in anchors {
        anchor.validate().expect("valid bounded anchor");
        let encoded = serde_json::to_vec(&anchor).expect("encode anchor");
        let decoded: ProjectionAnchor = serde_json::from_slice(&encoded).expect("decode anchor");
        assert_eq!(decoded, anchor);
    }
}

/// Rejects reversed ranges and oversized adapter-defined values.
#[test]
fn invalid_or_unbounded_anchors_are_rejected() {
    let reversed = ProjectionAnchor::SpreadsheetRange {
        worksheet_id: "worksheet-1".to_owned(),
        first_row: 4,
        first_column: 1,
        last_row: 2,
        last_column: 3,
    };
    assert!(reversed.validate().is_err());
    let oversized = ProjectionAnchor::Opaque {
        adapter: "example".to_owned(),
        anchor_kind: "shape".to_owned(),
        value: "v".repeat(4097),
    };
    assert!(oversized.validate().is_err());
    let oversized_worksheet = ProjectionAnchor::SpreadsheetCell {
        worksheet_id: "w".repeat(129),
        row: 1,
        column: 1,
    };
    assert!(oversized_worksheet.validate().is_err());
}
