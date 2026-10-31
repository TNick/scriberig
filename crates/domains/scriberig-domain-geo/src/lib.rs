//! Defines provider-neutral geospatial data and bounded feature collections.

mod collection;
mod derived_index;
mod geopackage;
mod operation;
mod query;
mod query_extension;
mod schema;
mod shapefile;
mod spatial_index;

pub use collection::{CollectionError, FeatureCollection, FeaturePage};
pub use derived_index::{GeoAttributeIndex, GeoIndexError};
pub use geopackage::{
    GeoPackageError, GeoPackageLayerInfo, GeoPackageReader,
    replace_feature_attribute_in_geopackage, replace_feature_geometry_in_geopackage,
    validate_geopackage_feature_edit, validate_geopackage_geometry_edit,
};
pub use operation::{GeoOperationError, replace_feature_geometry, set_feature_attribute};
pub use query::{GeoBounds, GeoFeatureHandle, GeoPredicate, GeoQueryError, GeoQueryPage};
pub use query_extension::{GeoAttributeQueryRequest, GeoAttributeSelector};
pub use schema::{
    Coordinate, CoordinateReferenceSystem, Feature, Field, FieldType, GeoDataset,
    GeoFieldDefinition, GeoLayer, GeoValue, Geometry, GeometryType, semantic_feature_id,
    semantic_field_id,
};
pub use shapefile::{
    ShapefileError, ShapefileLayer, ShapefileMemberPath, ShapefileReader, shapefile_member_path,
};
pub use spatial_index::GeoSpatialIndex;

#[cfg(test)]
mod tests {
    use scriberig_core::{EntityId, SnapshotId};

    use super::{
        Coordinate, CoordinateReferenceSystem, Feature, FeatureCollection, GeoBounds, GeoPredicate,
        GeoQueryError, GeoValue, Geometry, semantic_feature_id,
    };

    /// Creates a test feature with a native provider identifier.
    ///
    /// # Parameters
    /// - `dataset`: Owning semantic dataset identity.
    /// - `layer`: Owning semantic layer identity.
    /// - `native_id`: Provider-native feature identifier.
    /// - `area`: Exact numeric area representation.
    ///
    /// # Returns
    /// A semantic feature with the given dataset, layer, ID, and area.
    fn feature(dataset: EntityId, layer: EntityId, native_id: &str, area: &str) -> Feature {
        Feature {
            id: semantic_feature_id(dataset, layer, native_id),
            layer,
            native_id: Some(native_id.to_owned()),
            geometry: None,
            attributes: [("area".to_owned(), GeoValue::Number(area.to_owned()))]
                .into_iter()
                .collect(),
        }
    }

    /// Pages sparse feature rows and keeps unrelated feature identities stable.
    #[test]
    fn bounded_feature_edits_preserve_neighbor_identity() {
        let dataset = EntityId::new();
        let layer = EntityId::new();
        let first = feature(dataset, layer, "124", "10.5");
        assert_ne!(first.id.to_string(), "124");
        let mut collection =
            FeatureCollection::new([first, feature(dataset, layer, "125", "12")], 2)
                .expect("unique feature identities");
        let before = collection.page(0, 1).expect("first bounded page");
        assert_eq!(before.features.len(), 1);
        assert!(before.has_more);
        let neighbor = collection.page(1, 1).expect("second bounded page").features[0].id;
        let changed = feature(dataset, layer, "124", "11");
        let changed_id = changed.id;
        assert_eq!(collection.set(changed), changed_id);
        assert_eq!(
            collection.page(1, 1).expect("neighbor").features[0].id,
            neighbor
        );
    }

    /// Runs bounded feature queries with snapshot and CRS identity in each result.
    #[test]
    fn attribute_and_envelope_queries_are_bounded_and_snapshot_scoped() {
        let dataset = EntityId::new();
        let layer = EntityId::new();
        let crs = CoordinateReferenceSystem {
            authority: Some("EPSG".to_owned()),
            code: Some("4326".to_owned()),
            definition: None,
        };
        let mut first = feature(dataset, layer, "124", "10");
        first.geometry = Some(Geometry::Point(Coordinate {
            x: "2.5".to_owned(),
            y: "3.5".to_owned(),
            z: None,
            measure: None,
        }));
        let first_id = first.id;
        let mut second = feature(dataset, layer, "125", "10");
        second.geometry = Some(Geometry::Point(Coordinate {
            x: "20".to_owned(),
            y: "30".to_owned(),
            z: None,
            measure: None,
        }));
        let collection = FeatureCollection::new([first, second], 1).expect("collection");
        let snapshot = SnapshotId::new([7; 32]);
        let matches = collection
            .query(
                &GeoPredicate::AttributeEquals {
                    field: "area".to_owned(),
                    value: GeoValue::Number("10".to_owned()),
                },
                snapshot,
                Some(&crs),
                0,
                1,
            )
            .expect("bounded attribute query");
        assert_eq!(matches.matches.len(), 1);
        assert!(matches.has_more);
        assert_eq!(matches.matches[0].snapshot, snapshot);

        let spatial = collection
            .query(
                &GeoPredicate::EnvelopeOverlaps {
                    bounds: GeoBounds {
                        min_x: "0".to_owned(),
                        min_y: "0".to_owned(),
                        max_x: "5".to_owned(),
                        max_y: "5".to_owned(),
                        crs: crs.clone(),
                    },
                },
                snapshot,
                Some(&crs),
                0,
                1,
            )
            .expect("spatial query");
        assert_eq!(spatial.matches.len(), 1);
        assert_eq!(spatial.matches[0].feature, first_id);
        assert_eq!(
            collection.query(
                &GeoPredicate::EnvelopeOverlaps {
                    bounds: GeoBounds {
                        min_x: "0".to_owned(),
                        min_y: "0".to_owned(),
                        max_x: "5".to_owned(),
                        max_y: "5".to_owned(),
                        crs: CoordinateReferenceSystem {
                            authority: Some("EPSG".to_owned()),
                            code: Some("3857".to_owned()),
                            definition: None,
                        },
                    },
                },
                snapshot,
                Some(&crs),
                0,
                1,
            ),
            Err(GeoQueryError::CrsMismatch)
        );

        assert_large_decimal_bounds_keep_exact_comparison(dataset, layer, snapshot);
        assert_empty_collection_still_checks_crs(snapshot);
    }

    /// Verifies envelope queries compare large decimal coordinates without floating point loss.
    ///
    /// # Parameters
    /// - `dataset`: Semantic test dataset identity.
    /// - `layer`: Semantic test layer identity.
    /// - `snapshot`: Exact query snapshot.
    fn assert_large_decimal_bounds_keep_exact_comparison(
        dataset: EntityId,
        layer: EntityId,
        snapshot: SnapshotId,
    ) {
        let mut precise = feature(dataset, layer, "126", "13");
        precise.geometry = Some(Geometry::Point(Coordinate {
            x: "9007199254740993".to_owned(),
            y: "0".to_owned(),
            z: None,
            measure: None,
        }));
        let precise_collection = FeatureCollection::new([precise], 1).expect("precise feature");
        let exact_bounds = GeoBounds {
            min_x: "9007199254740992".to_owned(),
            min_y: "0".to_owned(),
            max_x: "9007199254740992".to_owned(),
            max_y: "0".to_owned(),
            crs: CoordinateReferenceSystem {
                authority: Some("EPSG".to_owned()),
                code: Some("4326".to_owned()),
                definition: None,
            },
        };
        let result = precise_collection
            .query(
                &GeoPredicate::EnvelopeOverlaps {
                    bounds: exact_bounds,
                },
                snapshot,
                Some(&CoordinateReferenceSystem {
                    authority: Some("EPSG".to_owned()),
                    code: Some("4326".to_owned()),
                    definition: None,
                }),
                0,
                1,
            )
            .expect("precise bounds query");
        assert_eq!(result.matches.len(), 0);
    }

    /// Verifies empty collections still validate geometry-query coordinate systems.
    ///
    /// # Parameters
    /// - `snapshot`: Exact query snapshot.
    fn assert_empty_collection_still_checks_crs(snapshot: SnapshotId) {
        let empty = FeatureCollection::new([], 1).expect("empty feature collection");
        assert_eq!(
            empty.query(
                &GeoPredicate::EnvelopeOverlaps {
                    bounds: GeoBounds {
                        min_x: "0".to_owned(),
                        min_y: "0".to_owned(),
                        max_x: "1".to_owned(),
                        max_y: "1".to_owned(),
                        crs: CoordinateReferenceSystem {
                            authority: Some("EPSG".to_owned()),
                            code: Some("4326".to_owned()),
                            definition: None,
                        },
                    },
                },
                snapshot,
                None,
                0,
                1,
            ),
            Err(GeoQueryError::CrsMismatch)
        );
    }
}
