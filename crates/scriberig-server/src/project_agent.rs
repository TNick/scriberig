//! Runs the deterministic project agent through the upstream agent loop.

use std::sync::{Arc, Mutex};

use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_agent_runtime::context::{
    BranchExecutionContext, ContextCompiler, ContextPolicy, ProjectSemanticScope,
};
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchInvocation, run_upstream_branch_invocation_blocking,
};
use scriberig_app::ApplicationContext;
use scriberig_core::{EntityId, SnapshotId};
use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::HttpState;
use crate::server::ApplicationService;

#[path = "project_agent_aggregate.rs"]
mod aggregate;
#[path = "project_agent_tools.rs"]
mod project_agent_tools;
use project_agent_tools::{faux_provider, project_tools};

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
struct ProjectAgentTool {
    /// Shared project store used by `ScribeRig` project operations.
    project_store: Arc<SqliteStore>,
    /// Project selected by the user.
    project: scriberig_core::ProjectId,
    /// Scope expected from the provider.
    expected: ObservedScope,
    /// Scope actually observed by the tool.
    observed: Arc<Mutex<Option<ObservedScope>>>,
    /// Result sink receiving the candidate identity created by proposal tools.
    candidate_id: Arc<Mutex<Option<String>>>,
}

impl Tool for ProjectAgentTool {
    /// Returns the stable provider tool identifier.
    ///
    /// The stable project tool name.
    /// # Returns
    /// The operation result.
    fn name(&self) -> &'static str {
        "scriberig.project"
    }

    /// Inspects the selected entity at the requested exact snapshot.
    ///
    /// - `_context`: Agent execution context.
    /// - `input`: Structured semantic inspection input.
    ///
    /// The selected entity's bounded semantic result.
    ///
    /// Returns a tool error when scope or project state is invalid.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
        let entity = self
            .project_store
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
    /// - `_context`: Agent execution context.
    /// - `name`: Provider tool name.
    /// - `arguments`: Structured proposal arguments.
    ///
    /// The normal migration-candidate tool result.
    ///
    /// Returns a tool error when proposal arguments or project validation fail.
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
            return Err(tool_error("unsupported project agent tool"));
        }
        if let Some(effects) = arguments.get("effects") {
            let result = aggregate::propose(
                &self.project_store,
                self.project,
                self.expected.entity,
                self.expected.snapshot,
                effects,
            )?;
            let candidate_id = result
                .handles
                .first()
                .cloned()
                .ok_or_else(|| tool_error("candidate identity missing"))?;
            self.candidate_id
                .lock()
                .map_err(|_| tool_error("candidate result lock poisoned"))?
                .replace(candidate_id);
            return Ok(result);
        }
        let text = arguments
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_scope("text"))?;
        let source = self
            .project_store
            .project_files(self.project)
            .map_err(|error| tool_error(&error.to_string()))?
            .into_iter()
            .find(|file| file.relative_path == "src/lib.rs")
            .ok_or_else(|| tool_error("source unavailable"))?;
        let edited = String::from_utf8_lossy(&source.bytes).replace("left + right", text);
        let candidate = ProjectService::new(Arc::clone(&self.project_store))
            .propose_candidate(self.project, self.expected.entity, edited)
            .map_err(|error| tool_error(&error.to_string()))?;
        self.candidate_id
            .lock()
            .map_err(|_| tool_error("candidate result lock poisoned"))?
            .replace(candidate.id.clone());
        Ok(ToolResult {
            entity: self.expected.entity,
            revision: None,
            snapshot: Some(self.expected.snapshot),
            object_type: Some("migration_candidate".to_owned()),
            summary: Some("candidate created".to_owned()),
            handles: vec![candidate.id],
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
    let candidate_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let branch = EntityId::new();
    let context = ApplicationContext::empty();
    let tool = Arc::new(ProjectAgentTool {
        project_store: Arc::clone(
            state
                .project_store
                .as_ref()
                .ok_or_else(|| (503, json!({"message": "project store unavailable"})))?,
        ),
        project,
        expected: ObservedScope { entity, snapshot },
        observed: Arc::clone(&observed),
        candidate_id: Arc::clone(&candidate_id),
    });
    let context_inputs = BranchExecutionContext::new(
        project.to_string(),
        branch.to_string(),
        "project candidate proposal",
        input.prompt.clone(),
    )
    .with_project_scope(ProjectSemanticScope {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        root: project.to_string(),
        selection: Some(entity.to_string()),
    });
    let compiled_context = ContextCompiler::new(ContextPolicy::default()).compile(&context_inputs);
    let (provider, model) = faux_provider(entity, snapshot, &accepted);
    let tools = project_tools(Arc::clone(&tool), branch, context.clone())
        .map_err(|error| (500, json!({"message": error})))?;
    let invocation = UpstreamBranchInvocation {
        branch,
        provider,
        model,
        request_secret: None,
        bearer: false,
        applied_settings: None,
        provider_profile: Some("scriberig-deterministic-project-agent".to_owned()),
        thinking_level: rpi_ai::ThinkingLevel::Off,
        context: compiled_context,
        tools,
    };
    let result = run_upstream_branch_invocation_blocking(invocation, input.prompt)
        .map_err(|failure| (500, json!({"message": failure.error.to_string()})))?;
    let candidate = candidate_id
        .lock()
        .map_err(|_| (500, json!({"message": "candidate result lock poisoned"})))?
        .clone()
        .ok_or_else(|| (500, json!({"message": "agent did not create a candidate"})))?;
    let store = Arc::clone(
        state
            .project_store
            .as_ref()
            .ok_or_else(|| (503, json!({"message": "project store unavailable"})))?,
    );
    let scope = observed
        .lock()
        .map_err(|_| (500, json!({"message": "scope lock poisoned"})))?
        .ok_or_else(|| {
            (
                500,
                json!({"message": "agent did not inspect selected scope"}),
            )
        })?;
    candidate_response(store, &candidate, scope, &result.output)
}

/// Builds the reviewed-candidate response after validating persisted state.
///
/// - `store`: Project store used by the project service.
/// - `candidate`: Exact candidate identity created by the branch tool.
/// - `scope`: Entity and snapshot observed by the inspect operation.
/// - `output`: Bounded final response from the upstream agent.
///
/// The response payload for the existing project-agent route.
///
/// # Returns
/// JSON containing the reviewed candidate and exact base/scope references.
///
/// # Errors
/// Returns an HTTP error when the candidate is missing or invalid.
fn candidate_response(
    store: Arc<SqliteStore>,
    candidate: &str,
    scope: ObservedScope,
    output: &str,
) -> Result<Value, (u16, Value)> {
    let detail = ProjectService::new(store)
        .candidate(candidate)
        .map_err(|error| (500, json!({"message": error.message})))?
        .ok_or_else(|| (500, json!({"message": "candidate disappeared"})))?;
    let (validation, apply_enabled) = match &detail.validation {
        scriberig_project::CandidateValidation::Pending => ("pending", false),
        scriberig_project::CandidateValidation::Passed => ("passed", true),
        scriberig_project::CandidateValidation::Failed { diagnostics } => {
            return Err((500, json!({"message": diagnostics.join("\n")})));
        }
    };
    Ok(json!({
        "candidate": candidate,
        "base": detail.base_snapshot.to_string(),
        "path": detail.path,
        "old": String::from_utf8_lossy(&detail.old),
        "new": String::from_utf8_lossy(&detail.new),
        "validation": validation,
        "applyEnabled": apply_enabled,
        "scope": {"entity": scope.entity.to_string(), "snapshot": scope.snapshot.to_string()},
        "output": output,
    }))
}

/// Reads the currently captured Rust source for a project.
///
/// - `state`: HTTP state owning project storage.
/// - `project`: Project whose source is requested.
///
/// The captured Rust source text.
///
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
/// - `value`: Missing input name.
///
/// A normalized tool error.
/// # Returns
/// The operation result.
fn invalid_scope(value: &str) -> ToolError {
    tool_error(&format!("missing {value}"))
}

/// Creates a provider-facing application error.
///
/// - `message`: Safe application diagnostic.
///
/// A normalized tool error.
/// # Returns
/// The operation result.
fn tool_error(message: &str) -> ToolError {
    ToolError::Application {
        category: "project_agent".to_owned(),
        message: message.to_owned(),
    }
}
