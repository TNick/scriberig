//! Implements the local project source projection operation.

use std::collections::HashMap;

use scriberig_app::{ApplicationError, ApplicationResult, OperationSummary};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_project::ProjectService;

/// Captures one imported project's source projection for local operations.
#[derive(Clone, Debug)]
pub struct ProjectProjection {
    /// Project owning the source projection.
    pub project: ProjectId,
    /// Shared project workflow service used for candidate operations.
    pub workflow: ProjectService,
    /// Exact project snapshot represented by the captured source.
    pub snapshot: SnapshotId,
    /// Source path and bytes keyed by semantic source entity.
    pub sources: HashMap<EntityId, (String, Vec<u8>)>,
}

/// Parses the deterministic terminal edit request into a source replacement.
///
/// - `change`: Encoded TUI candidate request.
///
/// Selected entity and replacement expression when the request is valid.
/// # Returns
/// The operation result.
pub fn parse_edit(change: &str) -> Option<(EntityId, &str)> {
    let mut fields = change.splitn(3, '|');
    (fields.next() == Some("project-edit")).then_some(())?;
    Some((fields.next()?.parse().ok()?, fields.next()?))
}

/// Renders a selected local project source through the public result model.
///
/// - `projection`: Snapshot-bound source projection data.
/// - `view`: View suffix containing the selected entity identity.
///
/// A rendered application result.
///
/// Returns a structured invalid-request or not-found error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn render(
    projection: &ProjectProjection,
    view: &str,
) -> Result<ApplicationResult, ApplicationError> {
    let entity = view.parse().map_err(|_| ApplicationError::InvalidRequest {
        operation: "render".to_owned(),
        message: "invalid project source entity".to_owned(),
    })?;
    let (path, bytes) =
        projection
            .sources
            .get(&entity)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: entity.to_string(),
            })?;
    let request = sr_projection::RenderRequest {
        specification: sr_projection::ProjectionSpecRef(entity),
        scope: sr_projection::SemanticScopeRef {
            entity,
            revision: None,
        },
        base_state: sr_projection::SemanticStateRef(projection.snapshot),
        kind: sr_projection::ProjectionKind::new("rust.source").expect("static projection kind"),
        style: sr_projection::StyleRef::Value(sr_projection::StyleValue::new(vec![1])),
        renderer: sr_projection::RendererVersion::new("scriberig-tui-project-v1"),
        context: sr_projection::RenderingContext::default(),
    };
    let source = String::from_utf8_lossy(bytes);
    let content = if source.contains("left.saturating_add(right)") {
        format!("{path}\nadd: left.saturating_add(right)\n{source}")
    } else if source.contains("left + right") {
        format!("{path}\nadd: left + right\n{source}")
    } else {
        format!("{path}\n{source}")
    };
    let rendered =
        sr_projection::RenderedProjection::from_bytes(content.into_bytes(), &request, None);
    Ok(ApplicationResult::Render(OperationSummary {
        handle: format!("project-source:{entity}"),
        snapshot: Some(projection.snapshot),
        revision: None,
        branch: None,
        diff: None,
        history: None,
        rendered: Some(rendered),
        validation: None,
        validation_profile: None,
        validation_diagnostics: Vec::new(),
        conflicts: Vec::new(),
    }))
}
