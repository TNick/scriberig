//! Implements the deterministic semantically scoped project agent tool.

#![allow(clippy::wildcard_imports)]

use super::*;
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{
    FakeProvider, FinishReason, ModelResponse, ProviderMetadata, ToolCall, ToolDefinition,
};
use scriberig_agent_runtime::tools::{ToolCatalog, ToolDescriptor, ToolExecutionMode};
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
    /// # Returns
    /// Stable provider-facing tool name.
    fn name(&self) -> &'static str {
        "scriberig.project"
    }

    /// Verifies and records the provider's semantic scope.
    ///
    /// # Parameters
    /// - `_context`: Trusted application context.
    /// - `input`: Provider semantic scope.
    ///
    /// # Returns
    /// Bounded semantic inspection result.
    ///
    /// # Errors
    /// Returns a scope mismatch error.
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
    /// # Parameters
    /// - `_context`: Trusted application context.
    /// - `name`: Provider control tool name.
    /// - `arguments`: Structured proposal arguments.
    ///
    /// # Returns
    /// Shared migration-candidate handle.
    ///
    /// # Errors
    /// Returns a structured proposal error.
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

/// Builds the deterministic provider script for the golden prompt.
///
/// # Parameters
/// - `entity`: Selected semantic entity.
/// - `snapshot`: Exact candidate base snapshot.
///
/// # Returns
/// Scripted provider model.
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
        descriptor("inspect", "Inspect selected semantic entity.", false),
        descriptor("project_propose", "Create migration candidate.", true),
    ]);
    ProviderModel::new(FakeProvider::from_responses(responses), "scriberig-golden")
        .with_catalog(catalog)
}

/// Creates one provider response for the deterministic script.
///
/// # Parameters
/// - `tool_call`: Optional provider tool call.
/// - `text`: Optional final provider text.
///
/// # Returns
/// Normalized provider response.
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
/// # Parameters
/// - `name`: Tool name.
/// - `description`: Tool description.
/// - `state_changing`: Whether the tool changes candidate state.
///
/// # Returns
/// Provider tool descriptor.
fn descriptor(name: &str, description: &str, state_changing: bool) -> ToolDescriptor {
    ToolDescriptor {
        definition: ToolDefinition {
            name: name.to_owned(),
            description: description.to_owned(),
            parameters: json!({"type": "object"}),
        },
        result_shape: "semantic result".to_owned(),
        state_changing,
        execution_mode: ToolExecutionMode::Atomic,
        required_capability: None,
    }
}
