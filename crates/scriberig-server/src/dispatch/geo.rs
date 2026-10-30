//! Maps typed geospatial feature reads through trusted application context.

use std::str::FromStr;

use scriberig_app::{ApplicationContext, CapabilityContext, Principal};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_protocol::{
    GeoAttributeEditRequest, GeoAttributeEditResponse, GeoCatalogRequest, GeoCatalogResponse,
    GeoFeatureQueryRequest, GeoFeatureQueryResponse, GeoFeaturesRequest, GeoFeaturesResponse,
    GeoGeometryEditRequest, GeoGeometryEditResponse, ProtocolError, ProtocolErrorCode, Response,
};

use crate::server::ApplicationService;

use super::geo_mapping::{map_crs, map_feature, map_layer, unmap_geometry};

/// Executes a bounded exact-snapshot typed geospatial query.
///
/// # Parameters
/// - `service`: Shared application boundary.
/// - `capabilities`: Host-resolved grants.
/// - `principal`: Authenticated caller.
/// - `request`: Exact source, layer, predicate, and page bounds.
/// - `correlation_id`: Protocol request identity.
///
/// # Returns
/// Matching features and exact source provenance, or a structured error.
pub(super) fn query<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &GeoFeatureQueryRequest,
    correlation_id: &str,
) -> Response {
    let (Ok(snapshot), Ok(project), Ok(layer)) = (
        SnapshotId::from_str(&request.snapshot),
        request.project.parse(),
        EntityId::from_str(&request.layer),
    ) else {
        return schema_error("invalid geospatial query identity", correlation_id);
    };
    let predicate = match &request.predicate {
        scriberig_protocol::GeoFeaturePredicate::AttributeEquals { field, value } => {
            scriberig_domain_geo::GeoPredicate::AttributeEquals {
                field: field.clone(),
                value: map_geo_value(value.clone()),
            }
        }
        scriberig_protocol::GeoFeaturePredicate::EnvelopeOverlaps { bounds } => {
            scriberig_domain_geo::GeoPredicate::EnvelopeOverlaps {
                bounds: scriberig_domain_geo::GeoBounds {
                    min_x: bounds.min_x.clone(),
                    min_y: bounds.min_y.clone(),
                    max_x: bounds.max_x.clone(),
                    max_y: bounds.max_y.clone(),
                    crs: scriberig_domain_geo::CoordinateReferenceSystem {
                        authority: bounds.crs.authority.clone(),
                        code: bounds.crs.code.clone(),
                        definition: bounds.crs.definition.clone(),
                    },
                },
            }
        }
    };
    let app_request = scriberig_app::GeoFeatureQuery {
        project,
        snapshot,
        path: request.path.clone(),
        layer,
        predicate,
        offset: request.offset,
        limit: request.limit,
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .query_geospatial_features(&context, &app_request)
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |result| {
                let page = result.page;
                Response::GeoFeatureQuery(GeoFeatureQueryResponse {
                    project: result.project.to_string(),
                    snapshot: page.snapshot.to_string(),
                    layer: page.layer.to_string(),
                    features: page.features.into_iter().map(map_feature).collect(),
                    has_more: page.has_more,
                    source_revision: page.provenance.source_revision.to_string(),
                    source_digest: format_digest(page.provenance.source_digest),
                    provider: page.provenance.provider,
                    layer_crs: page.provenance.layer_crs.map(map_crs),
                    strategy: "streamed_source_pages".to_owned(),
                    coordinate_transform: page.provenance.coordinate_transform,
                    diagnostics: page.provenance.diagnostics,
                })
            },
        )
}

/// Reads a bounded `GeoPackage` catalog through the shared Inspect operation.
///
/// # Parameters
/// - `service`: Application boundary receiving trusted request context.
/// - `capabilities`: Host-resolved project grants.
/// - `principal`: Authenticated caller established by the transport.
/// - `request`: Exact snapshot, source path, and catalog page bounds.
/// - `correlation_id`: Protocol request tracing identity.
///
/// # Returns
/// Typed provider-neutral catalog page or structured error.
pub(super) fn inspect<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &GeoCatalogRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .inspect_geopackage(
            &context,
            &request.project,
            &request.path,
            request.offset,
            request.limit,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |catalog| {
                Response::GeoCatalog(GeoCatalogResponse {
                    project: catalog.project.to_string(),
                    snapshot: catalog.snapshot.to_string(),
                    dataset: catalog.dataset.id.to_string(),
                    source_digest: format_digest(catalog.source_digest),
                    layers: catalog.layers.into_iter().map(map_layer).collect(),
                    total: catalog.total,
                    has_more: catalog.has_more,
                })
            },
        )
}

/// Reads a bounded feature page through the shared Inspect operation.
///
/// # Parameters
/// - `service`: Application boundary receiving the trusted request context.
/// - `capabilities`: Host-resolved capability grants, never client-supplied.
/// - `principal`: Authenticated caller established by the transport.
/// - `request`: Exact snapshot, source member, layer, and page bounds.
/// - `correlation_id`: Protocol request tracing identity.
///
/// Typed provider-neutral feature page or structured error.
///
/// # Returns
/// The typed feature page or a structured protocol error.
pub(super) fn read<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &GeoFeaturesRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .read_geopackage_features(
            &context,
            &request.project,
            &request.path,
            &request.layer,
            request.offset,
            request.limit,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |result| {
                Response::GeoFeatures(GeoFeaturesResponse {
                    project: result.project.to_string(),
                    snapshot: result.snapshot.to_string(),
                    layer: result.layer.to_string(),
                    features: result.page.features.into_iter().map(map_feature).collect(),
                    has_more: result.page.has_more,
                })
            },
        )
}

/// Proposes a typed geometry edit through exact-snapshot host authorization.
///
/// # Parameters
/// - `service`: Application boundary receiving trusted request context.
/// - `capabilities`: Host-resolved project grants.
/// - `principal`: Authenticated caller established by transport.
/// - `request`: Exact source snapshot, semantic target, and replacement geometry.
/// - `correlation_id`: Protocol request tracing identity.
///
/// # Returns
/// Candidate handle or structured protocol error.
pub(super) fn propose_geometry<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: GeoGeometryEditRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let Ok(layer) = EntityId::from_str(&request.layer) else {
        return schema_error("invalid layer identity", correlation_id);
    };
    let Ok(feature) = EntityId::from_str(&request.feature) else {
        return schema_error("invalid feature identity", correlation_id);
    };
    let geometry = unmap_geometry(request.geometry);
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .propose_geo_feature_geometry(
            &context,
            &request.project,
            &request.path,
            &layer.to_string(),
            &feature.to_string(),
            &geometry,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |candidate| {
                Response::GeoGeometryEdit(GeoGeometryEditResponse {
                    candidate: candidate.id,
                    project: candidate.project.to_string(),
                    base_snapshot: candidate.base_snapshot.to_string(),
                    layer: candidate.layer.to_string(),
                    feature: candidate.feature.to_string(),
                    path: candidate.path,
                })
            },
        )
}

/// Proposes a typed feature attribute edit through exact-snapshot host authorization.
///
/// # Parameters
/// - `service`: Application boundary receiving trusted request context.
/// - `capabilities`: Host-resolved project grants.
/// - `principal`: Authenticated caller established by transport.
/// - `request`: Exact source snapshot, semantic target, field, and replacement value.
/// - `correlation_id`: Protocol request tracing identity.
///
/// # Returns
/// Candidate handle or structured protocol error.
pub(super) fn propose_attribute<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: GeoAttributeEditRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let Ok(layer) = EntityId::from_str(&request.layer) else {
        return schema_error("invalid layer identity", correlation_id);
    };
    let Ok(feature) = EntityId::from_str(&request.feature) else {
        return schema_error("invalid feature identity", correlation_id);
    };
    let Ok(project) = request.project.parse::<scriberig_core::ProjectId>() else {
        return schema_error("invalid project identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    let field = request.field.clone();
    service
        .propose_geo_feature_attribute(
            &context,
            scriberig_project::GeoPackageFeatureAttributeEdit {
                project,
                base_snapshot: snapshot,
                path: request.path,
                layer,
                feature,
                field: request.field,
                value: map_geo_value(request.value),
            },
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |candidate| {
                Response::GeoAttributeEdit(GeoAttributeEditResponse {
                    candidate: candidate.id,
                    project: candidate.project.to_string(),
                    base_snapshot: candidate.base_snapshot.to_string(),
                    layer: candidate.layer.to_string(),
                    feature: candidate.feature.to_string(),
                    field,
                    path: candidate.path,
                })
            },
        )
}

/// Converts protocol attribute values into provider-neutral domain values.
///
/// # Parameters
/// - `value`: Typed protocol value.
///
/// # Returns
/// The equivalent geospatial domain value.
fn map_geo_value(value: scriberig_protocol::GeoValue) -> scriberig_domain_geo::GeoValue {
    match value {
        scriberig_protocol::GeoValue::Null => scriberig_domain_geo::GeoValue::Null,
        scriberig_protocol::GeoValue::Text(value) => scriberig_domain_geo::GeoValue::Text(value),
        scriberig_protocol::GeoValue::Number(value) => {
            scriberig_domain_geo::GeoValue::Number(value)
        }
        scriberig_protocol::GeoValue::Boolean(value) => {
            scriberig_domain_geo::GeoValue::Boolean(value)
        }
        scriberig_protocol::GeoValue::DateTime(value) => {
            scriberig_domain_geo::GeoValue::DateTime(value)
        }
    }
}

/// Encodes exact BLAKE3 digest bytes as lowercase hexadecimal text.
///
/// # Parameters
/// - `digest`: Exact source-member digest.
///
/// # Returns
/// A fixed-width lowercase hexadecimal digest.
fn format_digest(digest: [u8; 32]) -> String {
    blake3::Hash::from_bytes(digest).to_hex().to_string()
}

/// Creates a protocol schema error without exposing request contents.
///
/// # Parameters
/// - `message`: Safe schema diagnostic.
/// - `correlation_id`: Request tracing identity.
///
/// Structured protocol error response.
///
/// # Returns
/// A schema violation response with the request correlation identity.
fn schema_error(message: &str, correlation_id: &str) -> Response {
    Response::Error(ProtocolError {
        code: ProtocolErrorCode::SchemaViolation,
        message: message.to_owned(),
        details: None,
        correlation_id: Some(correlation_id.to_owned()),
    })
}
