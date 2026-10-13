//! Provides exact-snapshot semantic discovery and source rendering tools.

use scriberig_agent_core::{ToolError, ToolResult};
use scriberig_app::{ApplicationBackend, ApplicationContext};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use std::fmt::Write as _;

use crate::agent_tool::LocalAgentTool;
use crate::agent_tool_helpers::invalid_argument;

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Finds project-owned semantic entities in an exact snapshot.
    ///
    /// # Parameters
    /// - `context`: Trusted capability and snapshot scope.
    /// - `arguments`: Project, snapshot, query text, and bounded page limit.
    /// - `super`: Parent module containing the tool adapter implementation.
    ///
    /// # Returns
    /// A bounded list of stable semantic entity and source-path references.
    ///
    /// # Errors
    /// Returns invalid input, authorization, or stale-snapshot failures.
    pub(super) fn semantic_find(
        &self,
        context: &ApplicationContext,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let project = project_id(arguments)?;
        let snapshot = snapshot_id(arguments)?;
        require_context_snapshot(context, snapshot)?;
        let query = arguments
            .get("query")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid_argument("query"))?
            .to_lowercase();
        let limit = usize::try_from(
            arguments
                .get("limit")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(20)
                .min(50),
        )
        .unwrap_or(50);
        let service = self.project_service()?;
        let workspace = service
            .workspace(project)
            .map_err(|error| Self::project_error(&error))?;
        let symbols = service
            .find_symbols_at(project, snapshot, &query, limit)
            .map_err(|error| Self::project_error(&error))?;
        let mut matches = symbols;
        matches.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.entity.cmp(&right.entity))
        });
        let mut handles = matches
            .iter()
            .map(|symbol| {
                serde_json::json!({
                    "project": project.to_string(),
                    "snapshot": snapshot.to_string(),
                    "entity": symbol.entity.to_string(),
                    "path": symbol.path,
                    "kind": symbol.kind,
                    "name": symbol.name,
                })
                .to_string()
            })
            .collect::<Vec<_>>();
        handles.insert(0, format!("project:{project}"));
        handles.insert(1, format!("snapshot:{snapshot}"));
        let mut result = Self::result(
            format!(
                "semantic find in {} at {} matched {} entities for {:?}",
                workspace.project.name,
                snapshot,
                matches.len(),
                query
            ),
            handles,
        );
        result.entity = workspace.root_entity;
        result.snapshot = Some(snapshot);
        result.object_type = Some("semantic_find".to_owned());
        Ok(result)
    }

    /// Renders one project source through its exact semantic reference.
    ///
    /// # Parameters
    /// - `context`: Trusted capability and snapshot scope.
    /// - `arguments`: Project, snapshot, and source entity identities.
    /// - `super`: Parent module containing the tool adapter implementation.
    ///
    /// # Returns
    /// Source projection and stable digest/path handles, capped for model context.
    ///
    /// # Errors
    /// Returns invalid input, authorization, stale-snapshot, or ownership failures.
    pub(super) fn semantic_render(
        &self,
        context: &ApplicationContext,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let project = project_id(arguments)?;
        let snapshot = snapshot_id(arguments)?;
        require_context_snapshot(context, snapshot)?;
        let entity = arguments
            .get("entity")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("entity"))?
            .parse::<EntityId>()
            .map_err(|error| invalid_argument(&format!("invalid entity: {error}")))?;
        let projection = self
            .project_service()?
            .projection_at(project, snapshot, entity)
            .map_err(|error| Self::project_error(&error))?;
        let mut digest = String::with_capacity(64);
        for byte in projection.source_digest {
            write!(digest, "{byte:02x}").expect("writing to a string cannot fail");
        }
        let source = String::from_utf8_lossy(&projection.text);
        let rendered = source.chars().take(12_000).collect::<String>();
        let truncated = rendered.chars().count() < source.chars().count();
        let summary = format!(
            "{} {} ({}) at {}\n{}{}",
            projection.path,
            projection.name.as_deref().unwrap_or("source"),
            projection.kind,
            snapshot,
            rendered,
            if truncated {
                "\n[source preview truncated]"
            } else {
                ""
            }
        );
        let mut result = Self::result(
            summary,
            vec![
                format!("project:{project}"),
                format!("snapshot:{snapshot}"),
                format!("entity:{entity}"),
                format!("path:{}", projection.path),
                format!("source_digest:{digest}"),
            ],
        );
        result.entity = entity;
        result.snapshot = Some(snapshot);
        result.object_type = Some("source_projection".to_owned());
        Ok(result)
    }
}

/// Parses a project identity from named-tool arguments.
///
/// # Parameters
/// - `arguments`: Provider-supplied argument object.
///
/// # Returns
/// The requested project identity.
///
/// # Errors
/// Returns a structured invalid-argument error.
fn project_id(arguments: &serde_json::Value) -> Result<ProjectId, ToolError> {
    arguments
        .get("project")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_argument("project"))?
        .parse()
        .map_err(|error| invalid_argument(&format!("invalid project: {error}")))
}

/// Parses the exact snapshot required by a semantic source operation.
///
/// # Parameters
/// - `arguments`: Provider-supplied argument object.
///
/// # Returns
/// The requested immutable snapshot identity.
///
/// # Errors
/// Returns a structured invalid-argument error.
fn snapshot_id(arguments: &serde_json::Value) -> Result<SnapshotId, ToolError> {
    arguments
        .get("snapshot")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_argument("snapshot"))?
        .parse()
        .map_err(|error| invalid_argument(&format!("invalid snapshot: {error}")))
}

/// Requires semantic tool arguments to retain the trusted exact snapshot.
///
/// # Parameters
/// - `context`: Trusted application context.
/// - `snapshot`: Snapshot requested by the tool arguments.
///
/// # Returns
/// Unit when the argument matches the authoritative context.
///
/// # Errors
/// Returns a capability denial for missing or mismatched semantic scope.
fn require_context_snapshot(
    context: &ApplicationContext,
    snapshot: SnapshotId,
) -> Result<(), ToolError> {
    if context.snapshot == Some(snapshot) {
        return Ok(());
    }
    Err(ToolError::CapabilityDenied {
        action: "semantic_project_read".to_owned(),
        scope: format!("snapshot:{snapshot}"),
    })
}
