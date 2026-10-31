//! Renders bounded `GeoPackage` catalog metadata and provider-neutral feature pages.

use super::super::common::{append_value, render_load_state};
use crate::inspector::LoadState;
use scriberig_protocol::{GeoCatalogResponse, GeoFeaturesResponse, GeoGeometry, GeoValue};

#[path = "geo/query.rs"]
mod query;
pub(super) use query::render_query;

/// Renders layer catalog metadata including schema, CRS, count, and extent.
///
/// # Parameters
/// - `state`: Exact-snapshot catalog load state.
///
/// # Returns
/// Bounded Markdown describing the dataset and its layer catalog.
pub(super) fn render_catalog(state: &LoadState<GeoCatalogResponse>) -> String {
    render_load_state("Geospatial dataset", state, |catalog, output| {
        append_value(output, &format!("Dataset: {}", catalog.dataset));
        append_value(output, &format!("Snapshot: {}", catalog.snapshot));
        append_value(output, &format!("Source digest: {}", catalog.source_digest));
        append_value(output, &format!("Layers: {}", catalog.total));
        for layer in &catalog.layers {
            append_value(output, &format!("## {} · {}", layer.name, layer.id));
            append_value(output, &format!("Features: {}", layer.feature_count));
            append_value(output, &format!("Geometry: {:?}", layer.geometry_type));
            append_value(output, &format!("CRS: {}", crs_label(layer.crs.as_ref())));
            if let Some(extent) = &layer.extent {
                append_value(
                    output,
                    &format!(
                        "Extent: {}, {}, {}, {}",
                        extent[0], extent[1], extent[2], extent[3]
                    ),
                );
            }
            for field in &layer.fields {
                append_value(
                    output,
                    &format!(
                        "Field `{}` · {:?} · nullable={}",
                        field.name, field.field_type, field.nullable
                    ),
                );
            }
            for diagnostic in &layer.diagnostics {
                append_value(output, &format!("Diagnostic: {diagnostic}"));
            }
            append_value(output, &format!("Editable: {}", layer.editable));
            append_value(
                output,
                &format!("Inspect features: `/geo-features {}`", layer.id),
            );
        }
        if catalog.has_more {
            append_value(
                output,
                "More layers are available with `/geo <path> <offset>`.",
            );
        }
    })
}

/// Renders a bounded feature page without expanding large geometries.
///
/// # Parameters
/// - `state`: Exact-snapshot feature page load state.
/// - `selection`: Current semantic selection, if any.
///
/// # Returns
/// Bounded Markdown with feature IDs, geometry summaries, and typed attributes.
pub(super) fn render_features(
    state: &LoadState<GeoFeaturesResponse>,
    selection: Option<scriberig_core::EntityId>,
) -> String {
    render_load_state("Geospatial features", state, |page, output| {
        append_value(output, &format!("Layer: {}", page.layer));
        append_value(output, &format!("Snapshot: {}", page.snapshot));
        for feature in page.features.iter().take(40) {
            let marker = if selection.is_some_and(|id| id.to_string() == feature.id) {
                "› "
            } else {
                "  "
            };
            append_value(output, &format!("{marker}Feature {}", feature.id));
            if let Some(native_id) = &feature.native_id {
                append_value(output, &format!("  Source feature ID: {native_id}"));
            }
            if let Some(geometry) = &feature.geometry {
                append_value(
                    output,
                    &format!("  Geometry: {}", geometry_summary(geometry)),
                );
            }
            for (name, value) in feature.attributes.iter().take(40) {
                append_value(output, &format!("  {name} = {}", value_label(value)));
            }
        }
        if page.has_more {
            append_value(
                output,
                "More features are available with `/geo-features <layer-id> <offset>`.",
            );
        }
    })
}

/// Renders the lifecycle of a geospatial candidate when no shared review is active.
///
/// # Parameters
/// - `state`: Candidate load state.
///
/// # Returns
/// Bounded Markdown with candidate identity and validation state.
pub(super) fn render_edit(state: &LoadState<scriberig_app::GeoAttributeCandidate>) -> String {
    render_load_state("Geospatial attribute change", state, |candidate, output| {
        append_value(output, &format!("Candidate: {}", candidate.id));
        append_value(output, &format!("Feature: {}", candidate.feature));
        append_value(output, &format!("Field: {}", candidate.field));
        append_value(output, &format!("Validation: {:?}", candidate.validation));
        append_value(output, "Review this candidate in the shared Changes view.");
    })
}

/// Renders the lifecycle of a geospatial geometry candidate.
///
/// # Parameters
/// - `state`: Candidate load state.
///
/// # Returns
/// Bounded Markdown with candidate identity and validation state.
pub(super) fn render_geometry_edit(
    state: &LoadState<scriberig_app::GeoGeometryCandidate>,
) -> String {
    render_load_state("Geospatial geometry change", state, |candidate, output| {
        append_value(output, &format!("Candidate: {}", candidate.id));
        append_value(output, &format!("Feature: {}", candidate.feature));
        append_value(output, &format!("Layer: {}", candidate.layer));
        append_value(output, &format!("Validation: {:?}", candidate.validation));
        append_value(output, "Review this candidate in the shared Changes view.");
    })
}

/// Formats explicit CRS identity without interpreting its definition.
///
/// # Parameters
/// - `crs`: Optional provider-neutral CRS metadata.
///
/// # Returns
/// A short exact authority label or `unspecified`.
fn crs_label(crs: Option<&scriberig_protocol::GeoCrsView>) -> String {
    crs.map_or_else(
        || "unspecified".to_owned(),
        |value| match (&value.authority, &value.code) {
            (Some(authority), Some(code)) => format!("{authority}:{code}"),
            _ => value
                .definition
                .clone()
                .unwrap_or_else(|| "defined".to_owned()),
        },
    )
}

/// Summarizes geometry kind and vertex count without dumping coordinate arrays.
///
/// # Parameters
/// - `geometry`: Provider-neutral wire geometry.
///
/// # Returns
/// Geometry kind and bounded vertex count.
fn geometry_summary(geometry: &GeoGeometry) -> String {
    let (kind, count) = match geometry {
        GeoGeometry::Point(_) => ("point", 1),
        GeoGeometry::LineString(points) => ("line string", points.len()),
        GeoGeometry::Polygon(rings) => ("polygon", rings.iter().map(Vec::len).sum()),
        GeoGeometry::Collection(parts) => {
            ("collection", parts.iter().map(geometry_vertex_count).sum())
        }
    };
    format!("{kind}, {count} vertices")
}

/// Counts coordinate vertices in nested geometry.
///
/// # Parameters
/// - `geometry`: Geometry to count.
///
/// # Returns
/// Number of coordinate vertices.
fn geometry_vertex_count(geometry: &GeoGeometry) -> usize {
    let mut pending = vec![geometry];
    let mut count = 0_usize;
    while let Some(part) = pending.pop() {
        match part {
            GeoGeometry::Point(_) => count = count.saturating_add(1),
            GeoGeometry::LineString(points) => count = count.saturating_add(points.len()),
            GeoGeometry::Polygon(rings) => {
                count = count.saturating_add(rings.iter().map(Vec::len).sum::<usize>());
            }
            GeoGeometry::Collection(parts) => pending.extend(parts),
        }
    }
    count
}

/// Formats typed attribute values for terminal display.
///
/// # Parameters
/// - `value`: Typed geospatial attribute value.
///
/// # Returns
/// A bounded scalar value label.
fn value_label(value: &GeoValue) -> String {
    let text = match value {
        GeoValue::Null => "null".to_owned(),
        GeoValue::Text(text) | GeoValue::Number(text) | GeoValue::DateTime(text) => text.clone(),
        GeoValue::Boolean(value) => value.to_string(),
    };
    if text.chars().count() > 160 {
        format!("{}…", text.chars().take(160).collect::<String>())
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::render_catalog;
    use crate::inspector::LoadState;
    use scriberig_protocol::{
        GeoCatalogResponse, GeoCoordinate, GeoCrsView, GeoFeatureView, GeoFeaturesResponse,
        GeoFieldType, GeoFieldView, GeoGeometry, GeoGeometryType, GeoLayerView, GeoValue,
    };

    /// Shows `GeoLayer` CRS, schema, count, extent, and stable identity in the RPI Markdown view.
    #[test]
    fn catalog_view_renders_layer_summary_and_schema() {
        let state = LoadState::Ready(GeoCatalogResponse {
            project: "project-1".to_owned(),
            snapshot: "snapshot-1".to_owned(),
            dataset: "dataset-1".to_owned(),
            source_digest: "a".repeat(64),
            layers: vec![GeoLayerView {
                id: "layer-1".to_owned(),
                dataset: "dataset-1".to_owned(),
                name: "parcels".to_owned(),
                fields: vec![GeoFieldView {
                    name: "parcel_id".to_owned(),
                    field_type: GeoFieldType::Number,
                    nullable: false,
                }],
                crs: Some(GeoCrsView {
                    authority: Some("EPSG".to_owned()),
                    code: Some("4326".to_owned()),
                    definition: None,
                }),
                geometry_type: Some(GeoGeometryType::Polygon),
                feature_count: 37,
                extent: Some([
                    "1.0".to_owned(),
                    "2.0".to_owned(),
                    "3.0".to_owned(),
                    "4.0".to_owned(),
                ]),
                editable: false,
                diagnostics: vec![
                    "layer has package structures outside the safe geometry writer subset"
                        .to_owned(),
                ],
            }],
            total: 1,
            has_more: false,
        });
        let output = render_catalog(&state);
        for expected in [
            "parcels",
            "layer-1",
            "Features: 37",
            "EPSG:4326",
            "parcel_id",
            "Extent: 1.0, 2.0, 3.0, 4.0",
            "Editable: false",
            "Diagnostic: layer has package structures outside the safe geometry writer subset",
        ] {
            assert!(output.contains(expected), "missing {expected} in {output}");
        }
    }

    /// Shows feature IDs, typed attributes, and a compact geometry summary.
    #[test]
    fn feature_view_renders_bounded_semantic_details() {
        let page = GeoFeaturesResponse {
            project: "project-1".to_owned(),
            snapshot: "snapshot-1".to_owned(),
            layer: "layer-1".to_owned(),
            features: vec![GeoFeatureView {
                id: "feature-124".to_owned(),
                native_id: Some("provider-row-124".to_owned()),
                geometry: Some(GeoGeometry::LineString(vec![
                    GeoCoordinate {
                        x: "1.0".to_owned(),
                        y: "2.0".to_owned(),
                        z: None,
                        measure: None,
                    },
                    GeoCoordinate {
                        x: "3.0".to_owned(),
                        y: "4.0".to_owned(),
                        z: None,
                        measure: None,
                    },
                ])),
                attributes: [
                    ("parcel_id".to_owned(), GeoValue::Number("124".to_owned())),
                    ("name".to_owned(), GeoValue::Text("North field".to_owned())),
                ]
                .into(),
            }],
            has_more: true,
        };
        let output = super::render_features(&LoadState::Ready(page), None);
        for expected in [
            "feature-124",
            "provider-row-124",
            "line string, 2 vertices",
            "parcel_id = 124",
            "name = North field",
            "More features are available",
        ] {
            assert!(output.contains(expected), "missing {expected} in {output}");
        }
    }

    /// Shows bounded query matches and the provider, source, and CRS provenance.
    #[test]
    fn query_view_renders_match_and_evaluation_provenance() {
        let result = scriberig_protocol::GeoFeatureQueryResponse {
            project: "project-1".into(),
            snapshot: "snapshot-1".into(),
            layer: "layer-1".into(),
            features: vec![GeoFeatureView {
                id: "feature-124".into(),
                native_id: Some("124".into()),
                geometry: None,
                attributes: [("parcel_id".into(), GeoValue::Number("124".into()))].into(),
            }],
            has_more: false,
            source_revision: "revision-1".into(),
            source_digest: "ab".repeat(32),
            provider: "geopackage".into(),
            layer_crs: Some(GeoCrsView {
                authority: Some("EPSG".into()),
                code: Some("4326".into()),
                definition: Some("EPSG:4326".into()),
            }),
            strategy: "streamed_source_pages".into(),
            coordinate_transform: None,
            diagnostics: vec!["No CRS transformation applied.".into()],
        };
        let output = super::render_query(&LoadState::Ready(result));
        for expected in [
            "feature-124",
            "parcel_id = 124",
            "revision-1",
            "Provider: geopackage",
            "EPSG:4326",
            "Coordinate transform: none",
        ] {
            assert!(output.contains(expected), "missing {expected} in {output}");
        }
    }
}
