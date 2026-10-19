//! Implements the deterministic semantically scoped project agent tool.

#![allow(clippy::wildcard_imports)]

use super::*;
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{
    FakeProvider, FinishReason, ModelResponse, ProviderMetadata, ToolCall, ToolDefinition,
};
use scriberig_agent_runtime::tools::{
    ToolCatalog, ToolCategory, ToolDescriptor, ToolExecutionMode,
};
use scriberig_project::{ProjectSourceChange, ProjectSourceChangeKind};
use serde_json::{Value, json};

/// Executes the project agent's exact inspect and proposal calls.
pub struct ProjectAgentTool<'a> {
    /// Active project projection and shared workflow service.
    pub(super) projection: &'a ProjectProjection,
    /// Selected semantic entity.
    pub(super) entity: EntityId,
    /// Exact candidate base snapshot.
    pub(super) snapshot: SnapshotId,
}

impl Tool for ProjectAgentTool<'_> {
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

/// Builds the deterministic provider script for the golden prompt.
///
/// - `entity`: Selected semantic entity.
/// - `snapshot`: Exact candidate base snapshot.
///
/// Scripted provider model.
/// # Returns
/// The operation result.
pub fn golden_provider(entity: EntityId, snapshot: SnapshotId) -> ProviderModel<FakeProvider> {
    let responses = [
        Ok(response(
            Some(ToolCall {
                id: "inspect-add".to_owned(),
                name: "inspect".to_owned(),
                arguments: json!({
                    "entity": entity.to_string(),
                    "snapshot": snapshot.to_string(),
                    "detail": "detailed"
                }),
            }),
            None,
        )),
        Ok(response(
            Some(ToolCall {
                id: "propose-add".to_owned(),
                name: "project_propose".to_owned(),
                arguments: json!({"text": "left.saturating_add(right)"}),
            }),
            None,
        )),
        Ok(response(None, Some("Candidate ready for review."))),
    ];
    let catalog = ToolCatalog::new(vec![
        descriptor(
            "inspect",
            "Inspect selected semantic entity.",
            false,
            ToolCategory::SemanticRead,
        ),
        descriptor(
            "project_propose",
            "Create an exact-base project source candidate using the fallback projection path.",
            true,
            ToolCategory::OpaqueFallback,
        ),
    ]);
    ProviderModel::new(FakeProvider::from_responses(responses), "scriberig-golden")
        .with_catalog(catalog)
}

/// Creates one provider response for the deterministic script.
///
/// - `tool_call`: Optional provider tool call.
/// - `text`: Optional final provider text.
///
/// Normalized provider response.
/// # Returns
/// The operation result.
fn response(tool_call: Option<ToolCall>, text: Option<&str>) -> ModelResponse {
    ModelResponse {
        text: text.map(str::to_owned),
        tool_calls: tool_call.into_iter().collect(),
        finish_reason: if text.is_some() {
            FinishReason::Stop
        } else {
            FinishReason::ToolCall
        },
        usage: None,
        metadata: ProviderMetadata::default(),
    }
}

/// Creates a catalog entry for one deterministic provider tool.
///
/// - `name`: Tool name.
/// - `description`: Tool description.
/// - `state_changing`: Whether the tool changes candidate state.
/// - `category`: Trusted semantic operation category.
///
/// Provider tool descriptor.
/// # Returns
/// The operation result.
fn descriptor(
    name: &str,
    description: &str,
    state_changing: bool,
    category: ToolCategory,
) -> ToolDescriptor {
    ToolDescriptor {
        definition: ToolDefinition {
            name: name.to_owned(),
            description: description.to_owned(),
            parameters: json!({"type": "object"}),
        },
        category,
        result_shape: "semantic result".to_owned(),
        invocation_label: None,
        state_changing,
        execution_mode: ToolExecutionMode::Atomic,
        required_capability: None,
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
