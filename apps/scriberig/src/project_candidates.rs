//! Exposes durable project candidate operations to provider tool calls.

use scriberig_agent_core::{ToolError, ToolResult};
use scriberig_app::ApplicationContext;
use scriberig_core::{ProjectId, SnapshotId};
use scriberig_project::{
    CandidateValidation, ProjectSourceChange, ProjectSourceChangeKind, ProjectSourceEffectKind,
};

use crate::agent_tool::LocalAgentTool;
use crate::agent_tool_helpers::invalid_argument;

#[cfg(test)]
#[path = "project_candidates_tests.rs"]
mod tests;

impl<B: scriberig_app::ApplicationBackend> LocalAgentTool<'_, B> {
    /// Creates one exact-base project-wide source candidate.
    ///
    /// # Parameters
    /// - `context`: Trusted exact snapshot authorization context.
    /// - `arguments`: Project, base snapshot, and relative source effects.
    /// - `super`: Parent module containing the agent tool adapter.
    ///
    /// # Returns
    /// Stable candidate identity and effect count.
    ///
    /// # Errors
    /// Returns a scope, input, or shared project operation failure.
    pub(super) fn project_propose(
        &self,
        context: &ApplicationContext,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let project = parse_project(arguments)?;
        let snapshot = parse_snapshot(arguments)?;
        require_snapshot(context, snapshot)?;
        let effects = arguments
            .get("effects")
            .and_then(serde_json::Value::as_array)
            .filter(|effects| !effects.is_empty() && effects.len() <= 512)
            .ok_or_else(|| invalid_argument("effects"))?;
        let changes = effects
            .iter()
            .map(parse_effect)
            .collect::<Result<Vec<_>, _>>()?;
        let candidate = self
            .project_service()?
            .propose_project_change(project, snapshot, changes)
            .map_err(|error| Self::project_error(&error))?;
        let handles: Vec<String> = candidate
            .effects
            .iter()
            .map(|effect| {
                format!(
                    "{}:{}:{}",
                    effect_kind(effect.kind),
                    effect.path,
                    effect.new.as_ref().map_or(0, Vec::len)
                )
            })
            .collect();
        let mut result = Self::result(
            format!(
                "project-wide candidate {} created at base {} with {} effects",
                candidate.id,
                candidate.base_snapshot,
                candidate.effects.len()
            ),
            std::iter::once(format!("candidate:{}", candidate.id))
                .chain(std::iter::once(format!(
                    "snapshot:{}",
                    candidate.base_snapshot
                )))
                .chain(handles)
                .collect(),
        );
        result.snapshot = Some(snapshot);
        result.object_type = Some("project_candidate".to_owned());
        Ok(result)
    }

    /// Loads candidate identity, base, validation state, and changed paths.
    ///
    /// # Parameters
    /// - `arguments`: Stable candidate identity.
    /// - `super`: Parent module containing the agent tool adapter.
    ///
    /// # Returns
    /// Bounded candidate inspection result.
    ///
    /// # Errors
    /// Returns invalid input, storage failure, or missing candidate.
    pub(super) fn project_candidate(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let id = candidate_id(arguments)?;
        let candidate = self
            .project_service()?
            .candidate(id)
            .map_err(|error| Self::project_error(&error))?
            .ok_or_else(|| invalid_argument("candidate"))?;
        let mut result = Self::result(
            format!(
                "project candidate detail: candidate {} for project {} at base {} is {}",
                candidate.id,
                candidate.project,
                candidate.base_snapshot,
                validation_name(&candidate.validation)
            ),
            std::iter::once(format!("candidate:{}", candidate.id))
                .chain(std::iter::once(format!("project:{}", candidate.project)))
                .chain(std::iter::once(format!(
                    "snapshot:{}",
                    candidate.base_snapshot
                )))
                .chain(candidate.effects.iter().map(|effect| {
                    format!(
                        "{}:{}:old={}:new={}",
                        effect_kind(effect.kind),
                        effect.path,
                        effect.old.as_ref().map_or(0, Vec::len),
                        effect.new.as_ref().map_or(0, Vec::len)
                    )
                }))
                .collect(),
        );
        result.snapshot = Some(candidate.base_snapshot);
        result.object_type = Some("project_candidate_detail".to_owned());
        Ok(result)
    }

    /// Runs the shared isolated validator for one exact candidate.
    ///
    /// # Parameters
    /// - `arguments`: Stable candidate identity.
    /// - `super`: Parent module containing the agent tool adapter.
    ///
    /// # Returns
    /// Durable validation state and bounded diagnostics.
    ///
    /// # Errors
    /// Returns invalid input, stale-base, or workspace materialization failure.
    pub(super) fn project_validate_candidate(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let id = candidate_id(arguments)?;
        let candidate = self
            .project_service()?
            .validate_candidate(id)
            .map_err(|error| Self::project_error(&error))?;
        let summary = match &candidate.validation {
            CandidateValidation::Pending => "validation remains pending".to_owned(),
            CandidateValidation::Passed => format!("candidate {} validation passed", candidate.id),
            CandidateValidation::Failed { diagnostics } => format!(
                "candidate {} validation failed: {}",
                candidate.id,
                diagnostics
                    .iter()
                    .take(8)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        };
        let mut result = Self::result(
            summary,
            vec![
                format!("candidate:{}", candidate.id),
                format!("validation:{}", validation_name(&candidate.validation)),
            ],
        );
        result.snapshot = Some(candidate.base_snapshot);
        result.object_type = Some("candidate_validation".to_owned());
        Ok(result)
    }
}

/// Parses one project identity from provider arguments.
///
/// # Parameters
/// - `arguments`: Provider argument object.
///
/// # Returns
/// Project identity.
///
/// # Errors
/// Returns a structured invalid-argument failure.
fn parse_project(arguments: &serde_json::Value) -> Result<ProjectId, ToolError> {
    arguments
        .get("project")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_argument("project"))?
        .parse()
        .map_err(|_| invalid_argument("project"))
}

/// Parses the immutable candidate base snapshot.
///
/// # Parameters
/// - `arguments`: Provider argument object.
///
/// # Returns
/// Snapshot identity.
///
/// # Errors
/// Returns a structured invalid-argument failure.
fn parse_snapshot(arguments: &serde_json::Value) -> Result<SnapshotId, ToolError> {
    arguments
        .get("base_snapshot")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_argument("base_snapshot"))?
        .parse()
        .map_err(|_| invalid_argument("base_snapshot"))
}

/// Parses one project-relative source effect.
///
/// # Parameters
/// - `value`: One structured effect object.
///
/// # Returns
/// Shared project change request.
///
/// # Errors
/// Returns invalid path, kind, or content input.
fn parse_effect(value: &serde_json::Value) -> Result<ProjectSourceChange, ToolError> {
    let path = value
        .get("path")
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| invalid_argument("effect.path"))?
        .to_owned();
    let kind = match value
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_argument("effect.kind"))?
    {
        "create" => ProjectSourceChangeKind::Create,
        "update" => ProjectSourceChangeKind::Update,
        "delete" => ProjectSourceChangeKind::Delete,
        _ => return Err(invalid_argument("effect.kind")),
    };
    let content = match value.get("content") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(text)) => Some(text.as_bytes().to_vec()),
        Some(serde_json::Value::Array(bytes)) => Some(
            bytes
                .iter()
                .map(|byte| byte.as_u64().and_then(|value| u8::try_from(value).ok()))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| invalid_argument("effect.content"))?,
        ),
        _ => return Err(invalid_argument("effect.content")),
    };
    Ok(ProjectSourceChange {
        path,
        kind,
        content,
    })
}

/// Parses a durable candidate identity.
///
/// # Parameters
/// - `arguments`: Provider argument object.
///
/// # Returns
/// Candidate identifier text.
///
/// # Errors
/// Returns a structured invalid-argument failure.
fn candidate_id(arguments: &serde_json::Value) -> Result<&str, ToolError> {
    arguments
        .get("candidate")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid_argument("candidate"))
}

/// Requires a candidate operation to use the selected exact snapshot.
///
/// # Parameters
/// - `context`: Trusted operation context.
/// - `snapshot`: Snapshot requested by the provider.
///
/// # Returns
/// Unit when the snapshot matches the user-selected project scope.
///
/// # Errors
/// Returns a capability denial for an out-of-scope snapshot.
fn require_snapshot(context: &ApplicationContext, snapshot: SnapshotId) -> Result<(), ToolError> {
    if context.snapshot == Some(snapshot) {
        return Ok(());
    }
    Err(ToolError::CapabilityDenied {
        action: "project.candidate.propose".to_owned(),
        scope: format!("snapshot:{snapshot}"),
    })
}

/// Names a project effect kind for compact provider-facing evidence.
///
/// # Parameters
/// - `kind`: Shared project effect kind.
///
/// # Returns
/// Stable lowercase effect label.
fn effect_kind(kind: ProjectSourceEffectKind) -> &'static str {
    match kind {
        ProjectSourceEffectKind::Create => "create",
        ProjectSourceEffectKind::Update => "update",
        ProjectSourceEffectKind::Delete => "delete",
    }
}

/// Names the durable candidate validation state.
///
/// # Parameters
/// - `validation`: Candidate validation record.
///
/// # Returns
/// Stable concise state label.
fn validation_name(validation: &CandidateValidation) -> &'static str {
    match validation {
        CandidateValidation::Pending => "pending",
        CandidateValidation::Passed => "passed",
        CandidateValidation::Failed { .. } => "failed",
    }
}
