//! Runs the deterministic project agent through the provider/tool contracts.

use std::sync::{Arc, Mutex};

use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_agent_runtime::AgentRuntime;
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{
    FakeProvider, FinishReason, ModelResponse, ProviderMetadata, ToolCall, ToolDefinition,
};
use scriberig_agent_runtime::tools::{ToolCatalog, ToolDescriptor, ToolExecutionMode};
use scriberig_app::ApplicationContext;
use scriberig_core::{EntityId, SnapshotId};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::HttpState;
use crate::http::project_routes::propose;
use crate::server::ApplicationService;

/// Request for the deterministic project-agent workflow.
#[derive(Debug, Deserialize)]
struct AgentInput {
    /// Project receiving the candidate.
    project: String,
    /// Exact selected entity.
    entity: String,
    /// Exact selected snapshot.
    snapshot: String,
    /// User instruction sent to the provider.
    prompt: String,
}

/// Records the semantic scope observed by the provider-facing tool.
#[derive(Clone, Copy)]
struct ObservedScope {
    /// Entity received by the inspect tool.
    entity: EntityId,
    /// Snapshot received by the inspect tool.
    snapshot: SnapshotId,
}

/// Project-backed provider tool used by the deterministic golden provider.
struct ProjectAgentTool<'a, S> {
    /// HTTP state owning the public project operations.
    state: &'a HttpState<S>,
    /// Project selected by the user.
    project: scriberig_core::ProjectId,
    /// Scope expected from the provider.
    expected: ObservedScope,
    /// Scope actually observed by the tool.
    observed: Arc<Mutex<Option<ObservedScope>>>,
}

impl<S: ApplicationService> Tool for ProjectAgentTool<'_, S> {
    /// Returns the stable provider tool identifier.
    ///
    /// # Returns
    /// The stable project tool name.
    fn name(&self) -> &'static str {
        "scriberig.project"
    }

    /// Inspects the selected entity at the requested exact snapshot.
    ///
    /// # Parameters
    /// - `_context`: Agent execution context.
    /// - `input`: Structured semantic inspection input.
    ///
    /// # Returns
    /// The selected entity's bounded semantic result.
    ///
    /// # Errors
    /// Returns a tool error when scope or project state is invalid.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        let snapshot = input.snapshot.ok_or_else(|| invalid_scope("snapshot"))?;
        if input.entity != self.expected.entity || snapshot != self.expected.snapshot {
            return Err(ToolError::Application {
                category: "semantic_scope_mismatch".to_owned(),
                message: "agent inspected outside its selected entity and snapshot".to_owned(),
            });
        }
        self.observed
            .lock()
            .map_err(|_| tool_error("scope lock poisoned"))?
            .replace(ObservedScope {
                entity: input.entity,
                snapshot,
            });
        let store = self
            .state
            .project_store
            .as_ref()
            .ok_or_else(|| tool_error("project store unavailable"))?;
        let entity = store
            .project_entities(self.project)
            .map_err(|error| tool_error(&error.to_string()))?
            .into_iter()
            .find(|value| value.entity == input.entity)
            .ok_or_else(|| tool_error("selected entity unavailable"))?;
        Ok(ToolResult {
            entity: input.entity,
            revision: None,
            snapshot: Some(snapshot),
            object_type: Some(entity.kind),
            summary: entity.name,
            handles: vec![entity.relative_path],
        })
    }

    /// Creates the normal reviewed candidate through the project operation.
    ///
    /// # Parameters
    /// - `_context`: Agent execution context.
    /// - `name`: Provider tool name.
    /// - `arguments`: Structured proposal arguments.
    ///
    /// # Returns
    /// The normal migration-candidate tool result.
    ///
    /// # Errors
    /// Returns a tool error when proposal arguments or project validation fail.
    fn invoke_named(
        &self,
        _context: &ApplicationContext,
        name: &str,
        arguments: &Value,
    ) -> Result<ToolResult, ToolError> {
        if name != "project_propose" {
            return Err(tool_error("unsupported project agent tool"));
        }
        let text = arguments
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_scope("text"))?;
        let result = propose(
            self.state,
            &json!({
                "project": self.project.to_string(),
                "entity": self.expected.entity.to_string(),
                "text": text,
            })
            .to_string(),
        )
        .map_err(|(_, value)| {
            tool_error(
                value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("candidate failed"),
            )
        })?;
        let candidate = result
            .get("candidate")
            .and_then(Value::as_str)
            .ok_or_else(|| tool_error("candidate identity missing"))?;
        Ok(ToolResult {
            entity: self.expected.entity,
            revision: None,
            snapshot: Some(self.expected.snapshot),
            object_type: Some("migration_candidate".to_owned()),
            summary: Some("candidate created".to_owned()),
            handles: vec![candidate.to_owned()],
        })
    }
}

/// Runs the deterministic provider through inspect and candidate tools.
pub(super) fn agent<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let input: AgentInput =
        serde_json::from_str(body).map_err(|error| (400, json!({"message": error.to_string()})))?;
    let project = input
        .project
        .parse::<scriberig_core::ProjectId>()
        .map_err(|_| (400, json!({"message": "invalid project"})))?;
    let entity = input
        .entity
        .parse::<EntityId>()
        .map_err(|_| (400, json!({"message": "invalid entity"})))?;
    let snapshot = input
        .snapshot
        .parse::<SnapshotId>()
        .map_err(|_| (400, json!({"message": "invalid snapshot"})))?;
    let source = source_for(state, project)?;
    let accepted = source.replace("left + right", "left.saturating_add(right)");
    let observed = Arc::new(Mutex::new(None));
    let model = golden_model(entity, snapshot, &accepted);
    let tool = ProjectAgentTool {
        state,
        project,
        expected: ObservedScope { entity, snapshot },
        observed: Arc::clone(&observed),
    };
    let mut runtime = AgentRuntime::new(model, tool, 4);
    let result = runtime
        .run(
            &ApplicationContext::empty(),
            input.prompt,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .map_err(|error| (500, json!({"message": error.to_string()})))?;
    let scope = observed
        .lock()
        .map_err(|_| (500, json!({"message": "scope lock poisoned"})))?
        .ok_or_else(|| {
            (
                500,
                json!({"message": "agent did not inspect selected scope"}),
            )
        })?;
    let candidate = result
        .messages
        .iter()
        .rev()
        .find_map(|message| match message {
            scriberig_agent_runtime::AgentMessage::Tool(value)
                if value.object_type.as_deref() == Some("migration_candidate") =>
            {
                value.handles.first().cloned()
            }
            _ => None,
        })
        .ok_or_else(|| (500, json!({"message": "agent did not create a candidate"})))?;
    let store = state
        .project_store
        .as_ref()
        .ok_or_else(|| (500, json!({"message": "project catalog unavailable"})))?;
    let detail = scriberig_project::ProjectService::new(Arc::clone(store))
        .candidate(&candidate)
        .map_err(|error| (500, json!({"message": error.message})))?
        .ok_or_else(|| (500, json!({"message": "candidate disappeared"})))?;
    Ok(json!({
        "candidate": candidate,
        "base": detail.base_snapshot.to_string(),
        "path": detail.path,
        "old": String::from_utf8_lossy(&detail.old),
        "new": String::from_utf8_lossy(&detail.new),
        "validation": "passed",
        "applyEnabled": true,
        "scope": {"entity": scope.entity.to_string(), "snapshot": scope.snapshot.to_string()},
        "output": result.output,
    }))
}

/// Builds the deterministic provider model used by the product golden path.
///
/// # Parameters
/// - `entity`: Selected semantic entity.
/// - `snapshot`: Selected exact snapshot.
/// - `accepted`: Deterministic accepted source text.
///
/// # Returns
/// A provider model with inspect and proposal tool calls.
fn golden_model(
    entity: EntityId,
    snapshot: SnapshotId,
    accepted: &str,
) -> ProviderModel<FakeProvider> {
    let provider = FakeProvider::from_responses([
        Ok(response(
            Some(ToolCall {
                id: "inspect-add".to_owned(),
                name: "inspect".to_owned(),
                arguments: json!({"entity": entity.to_string(), "snapshot": snapshot.to_string(), "detail": "detailed"}),
            }),
            None,
        )),
        Ok(response(
            Some(ToolCall {
                id: "propose-add".to_owned(),
                name: "project_propose".to_owned(),
                arguments: json!({"text": accepted}),
            }),
            None,
        )),
        Ok(response(None, Some("Candidate ready for review."))),
    ]);
    let catalog = ToolCatalog::new(vec![
        tool_descriptor(
            "inspect",
            "Inspect selected semantic entity.",
            false,
            "semantic inspection",
        ),
        tool_descriptor(
            "project_propose",
            "Create a reviewed source candidate.",
            true,
            "migration candidate",
        ),
    ]);
    ProviderModel::new(provider, "scriberig-golden").with_catalog(catalog)
}

/// Creates one deterministic project-agent tool descriptor.
///
/// # Parameters
/// - `name`: Stable tool name.
/// - `description`: Human-readable tool description.
/// - `state_changing`: Whether the tool creates a candidate.
/// - `result_shape`: Public result classification.
///
/// # Returns
/// A provider tool descriptor.
fn tool_descriptor(
    name: &str,
    description: &str,
    state_changing: bool,
    result_shape: &str,
) -> ToolDescriptor {
    ToolDescriptor {
        definition: ToolDefinition {
            name: name.to_owned(),
            description: description.to_owned(),
            parameters: json!({"type": "object"}),
        },
        result_shape: result_shape.to_owned(),
        state_changing,
        execution_mode: ToolExecutionMode::Atomic,
        required_capability: None,
    }
}

/// Reads the currently captured Rust source for a project.
///
/// # Parameters
/// - `state`: HTTP state owning project storage.
/// - `project`: Project whose source is requested.
///
/// # Returns
/// The captured Rust source text.
///
/// # Errors
/// Returns an HTTP error tuple when storage or source lookup fails.
fn source_for<S: ApplicationService>(
    state: &HttpState<S>,
    project: scriberig_core::ProjectId,
) -> Result<String, (u16, Value)> {
    let store = state
        .project_store
        .as_ref()
        .ok_or_else(|| (503, json!({"message": "project store unavailable"})))?;
    store
        .project_files(project)
        .map_err(|error| (500, json!({"message": error.to_string()})))?
        .into_iter()
        .find(|file| file.relative_path == "src/lib.rs")
        .map(|file| String::from_utf8_lossy(&file.bytes).into_owned())
        .ok_or_else(|| (404, json!({"message": "source unavailable"})))
}

/// Creates a provider-facing invalid-input error.
///
/// # Parameters
/// - `value`: Missing input name.
///
/// # Returns
/// A normalized tool error.
fn invalid_scope(value: &str) -> ToolError {
    tool_error(&format!("missing {value}"))
}

/// Creates a provider-facing application error.
///
/// # Parameters
/// - `message`: Safe application diagnostic.
///
/// # Returns
/// A normalized tool error.
fn tool_error(message: &str) -> ToolError {
    ToolError::Application {
        category: "project_agent".to_owned(),
        message: message.to_owned(),
    }
}

/// Creates a normalized provider response.
///
/// # Parameters
/// - `tool_call`: Optional tool call emitted by the fake provider.
/// - `text`: Optional final provider text.
///
/// # Returns
/// A provider response with the corresponding finish reason.
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
