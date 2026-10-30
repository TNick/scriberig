//! Normalizes shared query inputs and results for geospatial views.

use super::{
    GeoCrsView, GeoFeatureView, geometry_from_application, hex_digest, value_from_application,
};

/// Converts shared query output while retaining exact source provenance.
///
/// # Parameters
/// - `result`: Exact-snapshot application query result.
///
/// # Returns
/// Protocol-shaped matches and complete provider-neutral provenance.
pub fn query_from_application(
    result: scriberig_app::GeoFeatureQueryResult,
) -> scriberig_protocol::GeoFeatureQueryResponse {
    let page = result.page;
    scriberig_protocol::GeoFeatureQueryResponse {
        project: result.project.to_string(),
        snapshot: page.snapshot.to_string(),
        layer: page.layer.to_string(),
        features: page.features.into_iter().map(feature_from_domain).collect(),
        has_more: page.has_more,
        source_revision: page.provenance.source_revision.to_string(),
        source_digest: hex_digest(&page.provenance.source_digest),
        provider: page.provenance.provider,
        layer_crs: page.provenance.layer_crs.map(|crs| GeoCrsView {
            authority: crs.authority,
            code: crs.code,
            definition: crs.definition,
        }),
        strategy: match page.provenance.strategy {
            scriberig_project::GeoQueryStrategy::StreamedSourcePages => {
                "streamed_source_pages".to_owned()
            }
        },
        coordinate_transform: page.provenance.coordinate_transform,
        diagnostics: page.provenance.diagnostics,
    }
}

/// Converts a domain predicate to its stable wire representation.
///
/// # Parameters
/// - `predicate`: Typed application query predicate.
///
/// # Returns
/// Equivalent provider-neutral protocol predicate.
#[must_use]
pub fn predicate_to_protocol(
    predicate: &scriberig_domain_geo::GeoPredicate,
) -> scriberig_protocol::GeoFeaturePredicate {
    match predicate {
        scriberig_domain_geo::GeoPredicate::AttributeEquals { field, value } => {
            scriberig_protocol::GeoFeaturePredicate::AttributeEquals {
                field: field.clone(),
                value: value_from_application(value.clone()),
            }
        }
        scriberig_domain_geo::GeoPredicate::EnvelopeOverlaps { bounds } => {
            scriberig_protocol::GeoFeaturePredicate::EnvelopeOverlaps {
                bounds: scriberig_protocol::GeoFeatureBounds {
                    min_x: bounds.min_x.clone(),
                    min_y: bounds.min_y.clone(),
                    max_x: bounds.max_x.clone(),
                    max_y: bounds.max_y.clone(),
                    crs: GeoCrsView {
                        authority: bounds.crs.authority.clone(),
                        code: bounds.crs.code.clone(),
                        definition: bounds.crs.definition.clone(),
                    },
                },
            }
        }
    }
}

/// Converts one semantic feature to its protocol representation.
///
/// # Parameters
/// - `feature`: Provider-neutral exact-snapshot feature.
///
/// # Returns
/// Wire-safe feature with exact values and coordinate text.
fn feature_from_domain(feature: scriberig_domain_geo::Feature) -> GeoFeatureView {
    GeoFeatureView {
        id: feature.id.to_string(),
        native_id: feature.native_id,
        geometry: feature.geometry.map(geometry_from_application),
        attributes: feature
            .attributes
            .into_iter()
            .map(|(name, value)| (name, value_from_application(value)))
            .collect(),
    }
}
