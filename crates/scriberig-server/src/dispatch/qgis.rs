//! Maps exact-snapshot QGIS project inspections to provider-neutral protocol views.

use std::str::FromStr;

use scriberig_app::{ApplicationContext, CapabilityContext, Principal};
use scriberig_core::SnapshotId;
use scriberig_protocol::{
    QgisLayerOrderRequest, QgisLayerOrderResponse, QgisLayerRenameRequest, QgisLayerRenameResponse,
    QgisProjectRequest, QgisProjectResponse, Response,
};

use crate::server::ApplicationService;

/// Inspects a QGIS project's layer references through shared application authority.
///
/// # Parameters
/// - `service`: Shared application boundary.
/// - `capabilities`: Host-resolved project grants.
/// - `principal`: Authenticated request principal.
/// - `request`: Exact project-relative path and snapshot.
/// - `correlation_id`: Protocol tracing identity.
///
/// # Returns
/// Typed project structure with linked semantic layer IDs or a structured error.
pub(super) fn inspect<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &QgisProjectRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid QGIS snapshot identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .inspect_qgis_project(&context, &request.project, &request.path)
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |result| {
                let model = result.qgis.project;
                Response::QgisProject(QgisProjectResponse {
                    project: result.owner.to_string(),
                    path: request.path.clone(),
                    snapshot: result.qgis.snapshot.to_string(),
                    revision: result.qgis.revision.to_string(),
                    id: model.id.to_string(),
                    title: model.title,
                    crs: model.crs.map(super::geo_mapping::map_crs),
                    layers: model
                        .layers
                        .into_iter()
                        .map(|layer| scriberig_protocol::QgisLayerReferenceView {
                            id: layer.id.to_string(),
                            name: layer.name,
                            data_source: layer.source.raw,
                            path: layer.source.path,
                            resolved_path: layer.resolved_path,
                            source_layer: layer.source_layer,
                            dataset: layer.dataset.map(|id| id.to_string()),
                            layer: layer.layer.map(|id| id.to_string()),
                            resolution_diagnostic: layer.resolution_diagnostic,
                            visible: layer.presentation.visible,
                            renderer: layer.presentation.renderer,
                            scale_range: layer.presentation.scale_range,
                        })
                        .collect(),
                    groups: model
                        .groups
                        .into_iter()
                        .map(|group| scriberig_protocol::QgisLayerGroupView {
                            id: group.id.to_string(),
                            name: group.name,
                            layers: group.layers.into_iter().map(|id| id.to_string()).collect(),
                        })
                        .collect(),
                })
            },
        )
}

/// Proposes one QGIS layer rename through the shared Propose authority boundary.
///
/// # Parameters
/// - `service`: Shared application boundary.
/// - `capabilities`: Host-resolved project grants.
/// - `principal`: Authenticated request principal.
/// - `request`: Exact project, snapshot, reference, and new name.
/// - `correlation_id`: Protocol tracing identity.
///
/// # Returns
/// Candidate handle or structured protocol error.
pub(super) fn rename<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &QgisLayerRenameRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid QGIS snapshot identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .propose_qgis_layer_rename(
            &context,
            &request.project,
            &request.reference,
            &request.new_name,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |candidate| {
                Response::QgisLayerRename(QgisLayerRenameResponse {
                    candidate: candidate.id,
                    project: candidate.project.to_string(),
                    base_snapshot: candidate.base_snapshot.to_string(),
                    reference: candidate.reference.to_string(),
                    path: candidate.path,
                })
            },
        )
}

/// Proposes a QGIS layer order through the shared project Propose boundary.
///
/// # Parameters
/// - `service`: Shared application boundary.
/// - `capabilities`: Host-resolved project grants.
/// - `principal`: Authenticated request principal.
/// - `request`: Exact project, snapshot, path, and full layer order.
/// - `correlation_id`: Protocol tracing identity.
///
/// # Returns
/// Candidate handle or structured protocol error.
pub(super) fn reorder<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &QgisLayerOrderRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid QGIS snapshot identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .propose_qgis_layer_order(
            &context,
            &request.project,
            &request.path,
            &request.ordered_references,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |candidate| {
                Response::QgisLayerOrder(QgisLayerOrderResponse {
                    candidate: candidate.id,
                    project: candidate.project.to_string(),
                    base_snapshot: candidate.base_snapshot.to_string(),
                    path: candidate.path,
                })
            },
        )
}

/// Creates a safe protocol schema failure.
///
/// # Parameters
/// - `message`: Safe schema diagnostic.
/// - `correlation_id`: Request tracing identity.
///
/// # Returns
/// Structured protocol error response.
fn schema_error(message: &str, correlation_id: &str) -> Response {
    Response::Error(scriberig_protocol::ProtocolError {
        code: scriberig_protocol::ProtocolErrorCode::SchemaViolation,
        message: message.to_owned(),
        details: None,
        correlation_id: Some(correlation_id.to_owned()),
    })
}
