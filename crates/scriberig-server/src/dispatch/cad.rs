//! Dispatches exact-snapshot CAD line edit candidates through shared authority.

use std::str::FromStr;

use scriberig_app::{ApplicationContext, CapabilityContext, Principal};
use scriberig_core::SnapshotId;
use scriberig_protocol::{
    CadCandidateValidation, CadCoordinate, CadDrawingInspectRequest, CadDrawingInspectResponse,
    CadDrawingSpace, CadEntitiesRequest, CadEntitiesResponse, CadEntityStyleRef, CadEntitySummary,
    CadGeometry, CadLayerChangeRequest, CadLayerChangeResponse, CadLayerSummary,
    CadLineEditRequest, CadLineEditResponse, ProtocolError, ProtocolErrorCode, Response,
};

use crate::server::ApplicationService;

/// Reads a CAD drawing using host-injected project Inspect authority.
///
/// - service: Shared application boundary.
/// - capabilities: Grants resolved by the trusted host.
/// - principal: Authenticated caller.
/// - request: Exact project, snapshot, and drawing identities.
/// - `correlation_id`: Request tracing identity.
///
/// Typed drawing summary or structured application error.
pub(super) fn inspect<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &CadDrawingInspectRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let context = cad_context(capabilities, principal, snapshot, correlation_id);
    service
        .inspect_cad_drawing(
            &context,
            &request.project,
            &request.snapshot,
            &request.drawing,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |view| {
                Response::CadDrawingInspect(CadDrawingInspectResponse {
                    project: view.project.to_string(),
                    snapshot: view.snapshot.to_string(),
                    drawing: view.drawing.to_string(),
                    units: view.units,
                    coordinate_reference: view.coordinate_reference,
                    layers: view
                        .layers
                        .into_iter()
                        .map(|layer| CadLayerSummary {
                            id: layer.id.to_string(),
                            name: layer.name,
                            visible: layer.visible,
                            locked: layer.locked,
                        })
                        .collect(),
                    entity_count: view.entity_count,
                    complete: view.complete,
                    diagnostics: view.diagnostics,
                })
            },
        )
}

/// Reads one CAD entity page using host-injected project Inspect authority.
///
/// - service: Shared application boundary.
/// - capabilities: Grants resolved by the trusted host.
/// - principal: Authenticated caller.
/// - request: Exact snapshot and bounded entity page.
/// - `correlation_id`: Request tracing identity.
///
/// Typed entity page or structured application error.
pub(super) fn read_entities<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &CadEntitiesRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let context = cad_context(capabilities, principal, snapshot, correlation_id);
    service
        .read_cad_entities(
            &context,
            &request.project,
            &request.snapshot,
            &request.drawing,
            request.offset,
            request.limit,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |page| {
                Response::CadEntities(CadEntitiesResponse {
                    project: page.project.to_string(),
                    snapshot: page.snapshot.to_string(),
                    drawing: page.drawing.to_string(),
                    entities: page.entities.into_iter().map(map_entity_summary).collect(),
                    total: page.total,
                    has_more: page.has_more,
                })
            },
        )
}

/// Converts one semantic CAD entity into its parser-neutral wire summary.
///
/// # Parameters
/// - `entity`: Semantic CAD entity from an immutable snapshot.
///
/// # Returns
/// Wire summary with stable Scriberig semantic IDs.
pub(super) fn map_entity_summary(entity: scriberig_domain_cad::DrawingEntity) -> CadEntitySummary {
    CadEntitySummary {
        id: entity.id.to_string(),
        layer: entity.layer.to_string(),
        space: map_space(entity.space),
        geometry: map_geometry(entity.geometry),
        style: entity.style.map(|style| CadEntityStyleRef {
            key: style.key,
            adapter: style.adapter,
        }),
    }
}

/// Maps one semantic drawing space to its protocol representation.
///
/// # Parameters
/// - `space`: Domain space with semantic layout identity.
///
/// # Returns
/// Parser-neutral protocol drawing space.
pub(super) fn map_space(space: scriberig_domain_cad::DrawingSpace) -> CadDrawingSpace {
    match space {
        scriberig_domain_cad::DrawingSpace::Model => CadDrawingSpace::Model,
        scriberig_domain_cad::DrawingSpace::Paper(id) => CadDrawingSpace::Paper(id.to_string()),
        scriberig_domain_cad::DrawingSpace::Named(name) => CadDrawingSpace::Named(name),
    }
}

/// Converts semantic CAD geometry into the stable parser-neutral wire enum.
///
/// - `geometry`: Typed semantic CAD geometry.
///
/// # Returns
/// Typed protocol geometry with semantic IDs encoded as strings.
fn map_geometry(geometry: scriberig_domain_cad::CadEntity) -> CadGeometry {
    use scriberig_domain_cad::CadEntity as Domain;
    match geometry {
        Domain::Line { start, end } => CadGeometry::Line {
            start: map_coordinate(start),
            end: map_coordinate(end),
        },
        Domain::Polyline { vertices, closed } => CadGeometry::Polyline {
            vertices: vertices.into_iter().map(map_coordinate).collect(),
            closed,
        },
        Domain::Circle { center, radius } => CadGeometry::Circle {
            center: map_coordinate(center),
            radius,
        },
        Domain::Arc {
            center,
            radius,
            start_angle,
            end_angle,
        } => CadGeometry::Arc {
            center: map_coordinate(center),
            radius,
            start_angle,
            end_angle,
        },
        Domain::Text {
            position,
            value,
            height,
        } => CadGeometry::Text {
            position: map_coordinate(position),
            value,
            height,
        },
        Domain::Dimension {
            definition_points,
            text,
        } => CadGeometry::Dimension {
            definition_points: definition_points.into_iter().map(map_coordinate).collect(),
            text,
        },
        Domain::BlockReference(reference) => CadGeometry::BlockReference {
            definition: reference.definition.to_string(),
            insertion: map_coordinate(reference.insertion),
            scale: reference.scale,
            rotation: reference.rotation,
        },
    }
}

/// Converts one exact domain coordinate into its wire DTO.
///
/// - `coordinate`: Domain coordinate in drawing units.
///
/// # Returns
/// Exact decimal coordinate DTO.
fn map_coordinate(coordinate: scriberig_domain_cad::Coordinate) -> CadCoordinate {
    CadCoordinate {
        x: coordinate.x,
        y: coordinate.y,
        z: coordinate.z,
    }
}

/// Creates trusted snapshot-bound context for one CAD read.
///
/// # Parameters
/// - `capabilities`: Grants resolved by the trusted host.
/// - `principal`: Authenticated caller, when present.
/// - `snapshot`: Exact requested snapshot.
/// - `correlation_id`: Protocol request tracing identity.
///
/// # Returns
/// Application context containing the exact snapshot and host grants.
pub(super) fn cad_context(
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    snapshot: SnapshotId,
    correlation_id: &str,
) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    }
}

/// Creates a candidate after resolving Propose authority from the trusted host.
///
/// - `service`: Shared application boundary.
/// - `capabilities`: Grants resolved by the host.
/// - `principal`: Authenticated caller.
/// - `request`: Exact snapshot and typed line geometry.
/// - `correlation_id`: Request tracing identity.
///
/// # Returns
/// Reviewable candidate response or structured application error.
pub(super) fn propose<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: CadLineEditRequest,
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
        .propose_cad_line_edit(
            &context,
            &request.project,
            &request.entity,
            request.start,
            request.end,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |candidate| {
                let validation = match candidate.validation {
                    scriberig_project::CandidateValidation::Pending => {
                        CadCandidateValidation::Pending
                    }
                    scriberig_project::CandidateValidation::Passed => {
                        CadCandidateValidation::Passed
                    }
                    scriberig_project::CandidateValidation::Failed { diagnostics } => {
                        CadCandidateValidation::Failed { diagnostics }
                    }
                };
                Response::CadLineEdit(CadLineEditResponse {
                    candidate: candidate.id,
                    project: candidate.project.to_string(),
                    base_snapshot: candidate.base_snapshot.to_string(),
                    path: candidate.path,
                    validation,
                })
            },
        )
}

/// Creates a candidate after resolving Propose authority from the trusted host.
///
/// # Parameters
/// - `service`: Shared application boundary.
/// - `capabilities`: Grants resolved by the host.
/// - `principal`: Authenticated caller.
/// - `request`: Exact snapshot and typed entity/layer identities.
/// - `correlation_id`: Request tracing identity.
///
/// # Returns
/// Reviewable candidate response or structured application error.
pub(super) fn propose_layer_change<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &CadLayerChangeRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let context = cad_context(capabilities, principal, snapshot, correlation_id);
    service
        .propose_cad_layer_change(
            &context,
            &request.project,
            &request.entity,
            &request.destination_layer,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |candidate| {
                let validation = match candidate.validation {
                    scriberig_project::CandidateValidation::Pending => {
                        CadCandidateValidation::Pending
                    }
                    scriberig_project::CandidateValidation::Passed => {
                        CadCandidateValidation::Passed
                    }
                    scriberig_project::CandidateValidation::Failed { diagnostics } => {
                        CadCandidateValidation::Failed { diagnostics }
                    }
                };
                Response::CadLayerChange(CadLayerChangeResponse {
                    candidate: candidate.id,
                    project: candidate.project.to_string(),
                    base_snapshot: candidate.base_snapshot.to_string(),
                    path: candidate.path,
                    validation,
                })
            },
        )
}

/// Creates a protocol schema error without exposing request contents.
///
/// - `message`: Safe schema diagnostic.
/// - `correlation_id`: Request tracing identity.
///
/// # Returns
/// Structured protocol error response.
pub(super) fn schema_error(message: &str, correlation_id: &str) -> Response {
    Response::Error(ProtocolError {
        code: ProtocolErrorCode::SchemaViolation,
        message: message.to_owned(),
        details: None,
        correlation_id: Some(correlation_id.to_owned()),
    })
}
