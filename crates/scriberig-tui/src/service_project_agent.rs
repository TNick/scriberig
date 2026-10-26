//! Implements the deterministic semantically scoped project agent tool.

#![allow(clippy::wildcard_imports)]

use super::*;
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_project::{ProjectSourceChange, ProjectSourceChangeKind};
use serde_json::Value;

/// Executes the project agent's exact inspect and proposal calls.
pub struct ProjectAgentTool {
    /// Active project projection and shared workflow service.
    pub(super) projection: ProjectProjection,
    /// Selected semantic entity.
    pub(super) entity: EntityId,
    /// Exact candidate base snapshot.
    pub(super) snapshot: SnapshotId,
}

impl Tool<ApplicationContext> for ProjectAgentTool {
    /// Returns the project-agent tool name.
    ///
    /// Stable provider-facing tool name.
    /// # Returns
    /// The operation result.
    fn name(&self) -> &'static str {
        "scriberig.project"
    }

    /// Verifies and records the provider's semantic scope.
    ///
    /// - `_context`: Trusted application context.
    /// - `input`: Provider semantic scope.
    ///
    /// Bounded semantic inspection result.
    ///
    /// Returns a scope mismatch error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        if input.entity != self.entity || input.snapshot != Some(self.snapshot) {
            return Err(ToolError::Application {
                category: "semantic_scope_mismatch".to_owned(),
                message: "agent left the selected entity or snapshot".to_owned(),
            });
        }
        Ok(ToolResult {
            entity: self.entity,
            revision: None,
            snapshot: Some(self.snapshot),
            object_type: Some("rust.function".to_owned()),
            summary: Some("add".to_owned()),
            handles: vec!["src/lib.rs".to_owned()],
        })
    }

    /// Creates a candidate through the shared project workflow.
    ///
    /// - `_context`: Trusted application context.
    /// - `name`: Provider control tool name.
    /// - `arguments`: Structured proposal arguments.
    ///
    /// Shared migration-candidate handle.
    ///
    /// Returns a structured proposal error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn invoke_named(
        &self,
        _context: &ApplicationContext,
        name: &str,
        arguments: &Value,
    ) -> Result<ToolResult, ToolError> {
        if name != "project_propose" {
            return Err(ToolError::Application {
                category: "project_agent".to_owned(),
                message: "unsupported project tool".to_owned(),
            });
        }
        if let Some(effects) = arguments.get("effects").and_then(Value::as_array) {
            let changes = effects
                .iter()
                .map(parse_project_change)
                .collect::<Result<Vec<_>, _>>()?;
            let candidate = self
                .projection
                .workflow
                .propose_project_change(self.projection.project, self.snapshot, changes)
                .map_err(|error| ToolError::Application {
                    category: error.code.to_owned(),
                    message: error.message,
                })?;
            return Ok(ToolResult {
                entity: self.entity,
                revision: None,
                snapshot: Some(self.snapshot),
                object_type: Some("migration_candidate".to_owned()),
                summary: Some(format!(
                    "candidate created with {} paths",
                    candidate.effects.len()
                )),
                handles: vec![candidate.id],
            });
        }
        let text = arguments
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Application {
                category: "project_agent".to_owned(),
                message: "proposal text missing".to_owned(),
            })?;
        let source =
            self.projection
                .sources
                .get(&self.entity)
                .ok_or_else(|| ToolError::Application {
                    category: "project_agent".to_owned(),
                    message: "selected source missing".to_owned(),
                })?;
        let edited = String::from_utf8_lossy(&source.1).replace("left + right", text);
        let candidate = self
            .projection
            .workflow
            .propose_candidate(self.projection.project, self.entity, edited)
            .map_err(|error| ToolError::Application {
                category: error.code.to_owned(),
                message: error.message,
            })?;
        Ok(ToolResult {
            entity: self.entity,
            revision: None,
            snapshot: Some(self.snapshot),
            object_type: Some("migration_candidate".to_owned()),
            summary: Some("candidate created".to_owned()),
            handles: vec![candidate.id],
        })
    }
}

/// Decodes one bounded provider source effect into the shared request type.
///
/// - `value`: Provider's structured effect object.
///
/// Shared project source change request.
///
/// Returns a provider tool error for missing or invalid effect fields.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn parse_project_change(value: &Value) -> Result<ProjectSourceChange, ToolError> {
    let path = value
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| project_argument_error("effect path missing"))?;
    let kind = match value.get("kind").and_then(Value::as_str) {
        Some("create") => ProjectSourceChangeKind::Create,
        Some("update") => ProjectSourceChangeKind::Update,
        Some("delete") => ProjectSourceChangeKind::Delete,
        _ => return Err(project_argument_error("effect kind invalid")),
    };
    let content = match value.get("content") {
        Some(Value::String(text)) => Some(text.as_bytes().to_vec()),
        Some(Value::Array(bytes)) => Some(
            bytes
                .iter()
                .map(|byte| byte.as_u64().and_then(|number| u8::try_from(number).ok()))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| project_argument_error("effect content bytes invalid"))?,
        ),
        Some(Value::Null) | None => None,
        _ => return Err(project_argument_error("effect content invalid")),
    };
    Ok(ProjectSourceChange {
        path: path.to_owned(),
        kind,
        content,
    })
}

/// Creates a structured provider argument error.
///
/// - `message`: Safe argument error detail.
///
/// Provider tool application error.
/// # Returns
/// The operation result.
fn project_argument_error(message: &str) -> ToolError {
    ToolError::Application {
        category: "project_agent_arguments".to_owned(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use scriberig_project::ProjectSourceChangeKind;
    use serde_json::json;

    use super::parse_project_change;

    /// Decodes provider effects into the shared project operation request.
    #[test]
    fn aggregate_effect_arguments_preserve_relative_path_and_content() {
        let effect = parse_project_change(&json!({
            "path": "src/generated.php",
            "kind": "create",
            "content": "<?php class Example {}"
        }))
        .expect("valid provider effect");
        assert_eq!(effect.path, "src/generated.php");
        assert_eq!(effect.kind, ProjectSourceChangeKind::Create);
        assert_eq!(
            effect.content.as_deref(),
            Some(b"<?php class Example {}".as_slice())
        );
    }
}
