//! Dispatches snapshot-bound CAD spatial queries through shared authority.

use std::str::FromStr;

use scriberig_app::{CapabilityContext, Principal};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_protocol::{
    CadBounds2D, CadBoundsQueryRequest, CadBoundsQueryResponse, CadDrawingSpace, Response,
};

use super::cad;
use crate::server::ApplicationService;

/// Executes one CAD bounds query using host-injected Inspect authority.
///
/// # Parameters
/// - service: Shared application service boundary.
/// - capabilities: Grants resolved by the trusted host.
/// - principal: Authenticated caller, when available.
/// - request: Exact-snapshot CAD space and envelope query.
/// - `correlation_id`: Request tracing identity.
///
/// # Returns
/// Bounded typed result or a structured protocol error.
pub(super) fn query<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: CadBoundsQueryRequest,
    correlation_id: &str,
) -> Response {
    let Ok(project) = ProjectId::from_str(&request.project) else {
        return cad::schema_error("invalid project identity", correlation_id);
    };
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return cad::schema_error("invalid snapshot identity", correlation_id);
    };
    let Ok(drawing) = EntityId::from_str(&request.drawing) else {
        return cad::schema_error("invalid drawing identity", correlation_id);
    };
    let Some(space) = parse_space(request.space) else {
        return cad::schema_error("invalid drawing space identity", correlation_id);
    };
    let context = cad::cad_context(capabilities, principal, snapshot, correlation_id);
    let query = scriberig_project::CadBoundsQueryRequest {
        project,
        snapshot,
        drawing,
        space,
        bounds: scriberig_project::CadBounds2D {
            min_x: request.bounds.min_x,
            min_y: request.bounds.min_y,
            max_x: request.bounds.max_x,
            max_y: request.bounds.max_y,
        },
        offset: request.offset,
        limit: request.limit,
    };
    service.read_cad_bounds_query(&context, query).map_or_else(
        |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
        |page| {
            Response::CadBoundsQuery(CadBoundsQueryResponse {
                project: page.project.to_string(),
                snapshot: page.snapshot.to_string(),
                drawing: page.drawing.to_string(),
                space: cad::map_space(page.space),
                bounds: CadBounds2D {
                    min_x: page.bounds.min_x,
                    min_y: page.bounds.min_y,
                    max_x: page.bounds.max_x,
                    max_y: page.bounds.max_y,
                },
                entities: page
                    .entities
                    .into_iter()
                    .map(cad::map_entity_summary)
                    .collect(),
                total: page.total,
                has_more: page.has_more,
                unbounded_entities: page.unbounded_entities,
                complete: page.complete,
            })
        },
    )
}

/// Converts a wire drawing space into its semantic representation.
///
/// # Parameters
/// - `space`: Protocol model, paper, or named-space value.
///
/// # Returns
/// Semantic drawing space, or None when a paper layout ID is malformed.
fn parse_space(space: CadDrawingSpace) -> Option<scriberig_domain_cad::DrawingSpace> {
    match space {
        CadDrawingSpace::Model => Some(scriberig_domain_cad::DrawingSpace::Model),
        CadDrawingSpace::Paper(layout) => Some(scriberig_domain_cad::DrawingSpace::Paper(
            EntityId::from_str(&layout).ok()?,
        )),
        CadDrawingSpace::Named(name) => Some(scriberig_domain_cad::DrawingSpace::Named(name)),
    }
}
