//! Dispatches bounded authored CAD text search through shared Inspect authority.

use std::str::FromStr;

use scriberig_app::{CapabilityContext, Principal};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_protocol::{CadTextFindRequest, CadTextFindResponse, CadTextMatch, Response};

use super::cad::{cad_context, map_entity_summary, schema_error};
use crate::server::ApplicationService;

/// Finds authored CAD text using host-injected project Inspect authority.
///
/// - `service`: Shared application boundary.
/// - `capabilities`: Grants resolved by the trusted host.
/// - `principal`: Authenticated caller.
/// - `request`: Exact-snapshot query and bounded page parameters.
/// - `correlation_id`: Request tracing identity.
///
/// Bounded semantic text matches or a structured application error.
pub(super) fn find_text<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &CadTextFindRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let Ok(project) = ProjectId::from_str(&request.project) else {
        return schema_error("invalid project identity", correlation_id);
    };
    let Ok(drawing) = EntityId::from_str(&request.drawing) else {
        return schema_error("invalid drawing identity", correlation_id);
    };
    let context = cad_context(capabilities, principal, snapshot, correlation_id);
    let search_request = scriberig_app::CadTextSearchRequest {
        project,
        snapshot,
        drawing,
        query: request.query.clone(),
        offset: request.offset,
        limit: request.limit,
    };
    service
        .find_cad_text(&context, &search_request)
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |search| {
                Response::CadTextFind(CadTextFindResponse {
                    project: search.project.to_string(),
                    snapshot: search.snapshot.to_string(),
                    drawing: search.drawing.to_string(),
                    matches: search
                        .matches
                        .into_iter()
                        .map(|matched| CadTextMatch {
                            entity: map_entity_summary(matched.entity),
                            text: matched.text,
                        })
                        .collect(),
                    total: search.total,
                    has_more: search.has_more,
                })
            },
        )
}
