//! Serves canonical project snapshot lineage and source-change provenance.

use std::sync::Arc;

use scriberig_app::{
    ApplicationContext, HistoryRequest, OperationProvider, SnapshotSelector, StoreOperationProvider,
};
use scriberig_project::ProjectService;
use serde_json::{Value, json};

use crate::http::HttpState;
use crate::server::ApplicationService;

/// Returns project history through the shared application history operation.
///
/// # Parameters
/// - `state`: HTTP application state.
/// - `body`: Project identity request body.
///
/// # Returns
/// Ordered immutable snapshots and candidate-linked transition details.
///
/// # Errors
/// Returns a structured request, project, or history error.
pub(super) fn history<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state
        .project_store
        .as_ref()
        .ok_or_else(super::project_routes_support::unavailable)?;
    let input: ProjectInput = serde_json::from_str(body)
        .map_err(|error| super::project_routes_support::invalid_request(&error))?;
    let project = input
        .project
        .parse()
        .map_err(|_| super::project_routes_support::invalid_project())?;
    let workspace = ProjectService::new(Arc::clone(store))
        .workspace(project)
        .map_err(|error| super::project_routes_support::project_error(&error))?;
    let summary = StoreOperationProvider::new(Arc::clone(store))
        .history(
            &ApplicationContext::empty(),
            &HistoryRequest {
                snapshot: SnapshotSelector::Exact(workspace.snapshot),
            },
        )
        .map_err(|error| (500, json!({"message": error.to_string()})))?;
    let transitions = summary.history.unwrap_or_default();
    let mut snapshots = Vec::new();
    let mut current = workspace.snapshot;
    let mut visited = std::collections::BTreeSet::new();
    while visited.insert(current) {
        snapshots.push(current.to_string());
        let Some(parent) = transitions
            .iter()
            .find(|transition| transition.result == current && transition.base != current)
            .map(|transition| transition.base)
        else {
            break;
        };
        current = parent;
    }
    let transition_details = transitions
        .iter()
        .map(|transition| {
            json!({
                "base": transition.base.to_string(),
                "result": transition.result.to_string(),
                "candidate": transition.source_change_id,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "project": project.to_string(),
        "snapshots": snapshots,
        "transitions": transition_details,
    }))
}

/// Request body for project-scoped history.
#[derive(serde::Deserialize)]
struct ProjectInput {
    /// Stable project identity.
    project: String,
}
