//! Renders bounded geospatial query results and provenance.

use super::super::super::common::{append_value, render_load_state};
use super::{crs_label, geometry_summary, value_label};
use crate::inspector::LoadState;

/// Renders bounded query matches together with exact source provenance.
///
/// # Parameters
/// - `state`: Exact-snapshot query lifecycle state.
///
/// # Returns
/// Bounded Markdown with matching features and query provenance.
pub fn render_query(state: &LoadState<scriberig_protocol::GeoFeatureQueryResponse>) -> String {
    render_load_state("Geospatial query", state, |result, output| {
        append_value(output, &format!("Layer: {}", result.layer));
        append_value(output, &format!("Snapshot: {}", result.snapshot));
        append_value(
            output,
            &format!("Source revision: {}", result.source_revision),
        );
        append_value(output, &format!("Source digest: {}", result.source_digest));
        append_value(output, &format!("Provider: {}", result.provider));
        append_value(output, &format!("Evaluation: {}", result.strategy));
        append_value(
            output,
            &format!("Layer CRS: {}", crs_label(result.layer_crs.as_ref())),
        );
        append_value(
            output,
            &format!(
                "Coordinate transform: {}",
                result.coordinate_transform.as_deref().unwrap_or("none")
            ),
        );
        for diagnostic in &result.diagnostics {
            append_value(output, &format!("Diagnostic: {diagnostic}"));
        }
        for feature in result.features.iter().take(40) {
            append_value(output, &format!("Feature {}", feature.id));
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
        if result.has_more {
            append_value(
                output,
                "More matches are available by increasing the `/geo-query` offset.",
            );
        }
    })
}
