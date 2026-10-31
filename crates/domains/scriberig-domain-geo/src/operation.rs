//! Validates typed feature changes against their owning layer schema.

use crate::{Feature, FieldType, GeoLayer, GeoValue, Geometry, GeometryType};

/// Identifies a rejected semantic geospatial operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoOperationError {
    /// Stable operation failure category.
    pub code: &'static str,
}

/// Creates a feature successor with replacement geometry after schema checks.
///
/// The source feature is never mutated. The returned value retains its semantic
/// identity and attributes so the shared change layer can create a new exact
/// immutable revision for only this feature. Replacement coordinates are
/// interpreted in the owning layer's CRS; this operation does not transform
/// coordinate systems or accept coordinates from another CRS.
///
/// # Parameters
/// - `layer`: Exact owning layer schema for the source feature.
/// - `feature`: Exact feature value to replace.
/// - `geometry`: New provider-neutral geometry.
///
/// # Returns
/// Successor feature value with the requested geometry.
///
/// # Errors
/// Returns an error when the feature belongs to another layer, the declared
/// geometry kind differs, or coordinate dimensions differ from the source.
pub fn replace_feature_geometry(
    layer: &GeoLayer,
    feature: &Feature,
    geometry: Geometry,
) -> Result<Feature, GeoOperationError> {
    if feature.layer != layer.id {
        return Err(operation_error("geo_feature_layer_mismatch"));
    }
    if layer
        .geometry_type
        .as_ref()
        .is_some_and(|expected| !geometry_matches(*expected, &geometry))
    {
        return Err(operation_error("geo_geometry_type_mismatch"));
    }
    if feature
        .geometry
        .as_ref()
        .and_then(geometry_dimensions)
        .zip(geometry_dimensions(&geometry))
        .is_some_and(|(old_dimensions, new_dimensions)| old_dimensions != new_dimensions)
    {
        return Err(operation_error("geo_geometry_dimension_mismatch"));
    }
    let mut successor = feature.clone();
    successor.geometry = Some(geometry);
    Ok(successor)
}

/// Creates a feature successor after validating one field value against its layer schema.
///
/// # Parameters
/// - `layer`: Exact owning layer schema for the source feature.
/// - `feature`: Exact feature value to replace.
/// - `field_name`: Authored field name in the layer schema.
/// - `value`: New typed attribute value.
///
/// # Returns
/// Successor feature retaining its identity and all unrelated data.
///
/// # Errors
/// Returns an error for a foreign feature, missing field, nullability violation,
/// mismatched scalar type, malformed numeric text, or unsupported binary fields.
pub fn set_feature_attribute(
    layer: &GeoLayer,
    feature: &Feature,
    field_name: &str,
    value: GeoValue,
) -> Result<Feature, GeoOperationError> {
    if feature.layer != layer.id {
        return Err(operation_error("geo_feature_layer_mismatch"));
    }
    let field = layer
        .fields
        .iter()
        .find(|field| field.name == field_name)
        .ok_or_else(|| operation_error("geo_field_not_found"))?;
    if matches!(value, GeoValue::Null) {
        if !field.nullable {
            return Err(operation_error("geo_field_null_not_allowed"));
        }
    } else if !attribute_type_matches(field.field_type, &value) {
        return Err(operation_error("geo_field_type_mismatch"));
    }
    let mut successor = feature.clone();
    successor.attributes.insert(field_name.to_owned(), value);
    Ok(successor)
}

/// Checks one authored value against the domain scalar type declared by its field.
///
/// # Parameters
/// - `field_type`: Semantic scalar type from the layer schema.
/// - `value`: Authored typed value.
///
/// # Returns
/// True when the value is compatible with the declared scalar type.
fn attribute_type_matches(field_type: FieldType, value: &GeoValue) -> bool {
    match (field_type, value) {
        (FieldType::Text, GeoValue::Text(_))
        | (FieldType::Boolean, GeoValue::Boolean(_))
        | (FieldType::DateTime, GeoValue::DateTime(_)) => true,
        (FieldType::Number, GeoValue::Number(value)) => valid_decimal_text(value),
        _ => false,
    }
}

/// Checks signed plain-decimal notation without converting or rounding the value.
///
/// # Parameters
/// - `value`: Decimal text from a typed numeric attribute.
///
/// # Returns
/// True when the text contains digits and at most one decimal point.
fn valid_decimal_text(value: &str) -> bool {
    let unsigned = value
        .strip_prefix('-')
        .or_else(|| value.strip_prefix('+'))
        .unwrap_or(value);
    let mut parts = unsigned.split('.');
    let integer = parts.next().unwrap_or_default();
    let fraction = parts.next();
    !integer.is_empty()
        && integer.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.is_none_or(|digits| digits.bytes().all(|byte| byte.is_ascii_digit()))
        && parts.next().is_none()
}

/// Checks the geometry variant against a layer's declared kind.
///
/// # Parameters
/// - `expected`: Declared layer geometry kind.
/// - `geometry`: Proposed feature geometry.
///
/// # Returns
/// `true` when the geometry has the declared outer kind.
fn geometry_matches(expected: GeometryType, geometry: &Geometry) -> bool {
    matches!(
        (expected, geometry),
        (GeometryType::Point, Geometry::Point(_))
            | (GeometryType::LineString, Geometry::LineString(_))
            | (GeometryType::Polygon, Geometry::Polygon(_))
            | (GeometryType::GeometryCollection, Geometry::Collection(_))
    )
}

/// Returns coordinate dimensionality if all positions have consistent Z/M values.
///
/// # Parameters
/// - `geometry`: Geometry whose coordinates are inspected.
///
/// # Returns
/// Coordinate dimension count, or `None` for empty/inconsistent geometry.
fn geometry_dimensions(geometry: &Geometry) -> Option<u8> {
    let mut dimensions = None;
    for coordinate in coordinates(geometry) {
        let current = u8::from(coordinate.z.is_some()) + 2 * u8::from(coordinate.measure.is_some());
        if dimensions.is_some_and(|previous| previous != current) {
            return None;
        }
        dimensions = Some(current);
    }
    dimensions
}

/// Flattens coordinates from a nested geometry without allocating a second collection.
///
/// # Parameters
/// - `geometry`: Geometry to traverse.
///
/// # Returns
/// Iterator over all coordinate tuples in source order.
fn coordinates(geometry: &Geometry) -> Box<dyn Iterator<Item = &crate::Coordinate> + '_> {
    match geometry {
        Geometry::Point(point) => Box::new(std::iter::once(point)),
        Geometry::LineString(points) => Box::new(points.iter()),
        Geometry::Polygon(rings) => Box::new(rings.iter().flatten()),
        Geometry::Collection(items) => Box::new(items.iter().flat_map(coordinates)),
    }
}

/// Creates a stable typed geospatial operation error.
///
/// # Parameters
/// - `code`: Stable failure category.
///
/// # Returns
/// Operation error without source data.
fn operation_error(code: &'static str) -> GeoOperationError {
    GeoOperationError { code }
}

#[cfg(test)]
mod tests {
    //! Verifies semantic geometry and attribute successors preserve exact identity.

    use std::collections::BTreeMap;

    use scriberig_core::EntityId;

    use crate::{
        Coordinate, Feature, Field, FieldType, GeoLayer, GeoValue, Geometry, GeometryType,
        replace_feature_geometry, set_feature_attribute,
    };

    /// Replaces one feature geometry while preserving all unrelated feature data.
    #[test]
    fn geometry_replacement_returns_exact_feature_successor() {
        let layer = layer(GeometryType::Point);
        let source = feature(layer.id);
        let replacement = Geometry::Point(Coordinate {
            x: "12.5".to_owned(),
            y: "13.5".to_owned(),
            z: None,
            measure: None,
        });

        let successor = replace_feature_geometry(&layer, &source, replacement.clone())
            .expect("same kind and dimensions");

        assert_eq!(successor.id, source.id);
        assert_eq!(successor.native_id, source.native_id);
        assert_eq!(successor.attributes, source.attributes);
        assert_eq!(successor.geometry, Some(replacement));
        assert_eq!(source.geometry, Some(Geometry::Point(coordinate("1", "2"))));
    }

    /// Rejects a geometry whose kind differs from the declared layer schema.
    #[test]
    fn geometry_replacement_rejects_wrong_kind_and_dimensions() {
        let layer = layer(GeometryType::Point);
        let source = feature(layer.id);
        let wrong_kind = replace_feature_geometry(
            &layer,
            &source,
            Geometry::LineString(vec![coordinate("0", "0"), coordinate("1", "1")]),
        )
        .expect_err("declared point layer rejects a line");
        assert_eq!(wrong_kind.code, "geo_geometry_type_mismatch");
        let wrong_dimension = replace_feature_geometry(
            &layer,
            &source,
            Geometry::Point(crate::Coordinate {
                x: "3".to_owned(),
                y: "4".to_owned(),
                z: Some("5".to_owned()),
                measure: None,
            }),
        )
        .expect_err("2D source rejects 3D replacement");
        assert_eq!(wrong_dimension.code, "geo_geometry_dimension_mismatch");
    }

    /// Validates attribute edits against field type and nullability while preserving other values.
    #[test]
    fn attribute_edit_validates_schema_and_preserves_unrelated_attributes() {
        let mut layer = layer(GeometryType::Point);
        layer.fields = vec![
            Field {
                name: "area".to_owned(),
                field_type: FieldType::Number,
                nullable: false,
            },
            Field {
                name: "note".to_owned(),
                field_type: FieldType::Text,
                nullable: true,
            },
        ];
        let mut source = feature(layer.id);
        source
            .attributes
            .insert("area".to_owned(), GeoValue::Number("10.5".to_owned()));
        source
            .attributes
            .insert("note".to_owned(), GeoValue::Text("surveyed".to_owned()));

        let successor = set_feature_attribute(
            &layer,
            &source,
            "area",
            GeoValue::Number("12.25".to_owned()),
        )
        .expect("valid numeric attribute successor");

        assert_eq!(successor.id, source.id);
        assert_eq!(successor.geometry, source.geometry);
        assert_eq!(
            successor.attributes["area"],
            GeoValue::Number("12.25".into())
        );
        assert_eq!(successor.attributes["note"], source.attributes["note"]);
        assert_eq!(source.attributes["area"], GeoValue::Number("10.5".into()));
        assert_eq!(
            set_feature_attribute(&layer, &source, "area", GeoValue::Null)
                .expect_err("non-nullable field rejects null")
                .code,
            "geo_field_null_not_allowed"
        );
        assert_eq!(
            set_feature_attribute(&layer, &source, "area", GeoValue::Text("large".into()))
                .expect_err("field rejects a mismatched scalar type")
                .code,
            "geo_field_type_mismatch"
        );
        assert_eq!(
            set_feature_attribute(&layer, &source, "area", GeoValue::Number("1e3".into()),)
                .expect_err("field rejects exponent-form decimal text")
                .code,
            "geo_field_type_mismatch"
        );
    }

    /// Creates a typed point layer for geometry operation tests.
    ///
    /// # Parameters
    /// - `geometry_type`: Declared geometry kind for the test layer.
    ///
    /// # Returns
    /// Layer with no fields or CRS constraints.
    fn layer(geometry_type: GeometryType) -> GeoLayer {
        GeoLayer {
            id: EntityId::new(),
            dataset: EntityId::new(),
            name: "parcels".to_owned(),
            fields: Vec::new(),
            crs: None,
            geometry_type: Some(geometry_type),
        }
    }

    /// Creates an exact decimal coordinate pair.
    ///
    /// # Parameters
    /// - `x`: Exact decimal x ordinate.
    /// - `y`: Exact decimal y ordinate.
    ///
    /// # Returns
    /// Two-dimensional coordinate.
    fn coordinate(x: &str, y: &str) -> Coordinate {
        Coordinate {
            x: x.to_owned(),
            y: y.to_owned(),
            z: None,
            measure: None,
        }
    }

    /// Creates a point feature with an unrelated attribute to preserve.
    ///
    /// # Parameters
    /// - `layer`: Owning layer identity.
    ///
    /// # Returns
    /// Feature with exact native ID, point, and parcel identifier.
    fn feature(layer: EntityId) -> Feature {
        Feature {
            id: EntityId::new(),
            layer,
            native_id: Some("124".to_owned()),
            geometry: Some(Geometry::Point(coordinate("1", "2"))),
            attributes: BTreeMap::from([(
                "parcel_id".to_owned(),
                GeoValue::Text("124".to_owned()),
            )]),
        }
    }
}
