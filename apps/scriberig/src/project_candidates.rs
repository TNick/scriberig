//! Exposes durable project candidate operations to provider tool calls.

use scriberig_agent_core::{ToolError, ToolResult};
use scriberig_app::ApplicationContext;
use scriberig_core::{ProjectId, SnapshotId};
use scriberig_project::{
    CandidateValidation, CandidateValidationProfile, ProjectSourceChange, ProjectSourceChangeKind,
    ProjectSourceEffectKind,
};

use crate::agent_tool::LocalAgentTool;
use crate::agent_tool_helpers::invalid_argument;

#[cfg(test)]
#[path = "project_candidates_tests.rs"]
mod tests;

impl<B: scriberig_app::ApplicationBackend> LocalAgentTool<'_, B> {
    /// Creates one exact-base project-wide source candidate.
    ///
    /// - `context`: Trusted exact snapshot authorization context.
    /// - `arguments`: Project, base snapshot, and relative source effects.
    /// - `super`: Parent module containing the agent tool adapter.
    ///
    /// Stable candidate identity and effect count.
    ///
    /// Returns a scope, input, or shared project operation failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
                .chain(
                    candidate
                        .validation_profile
                        .as_ref()
                        .map(|profile| format!("validation_profile:{profile}")),
                )
                .chain(handles)
                .collect(),
        );
        result.snapshot = Some(snapshot);
        result.object_type = Some("project_candidate".to_owned());
        Ok(result)
    }

    /// Loads candidate identity, base, validation state, and changed paths.
    ///
    /// - `arguments`: Candidate identity and optional registered profile identifier.
    /// - `super`: Parent module containing the agent tool adapter.
    ///
    /// Bounded candidate inspection result.
    ///
    /// Returns invalid input, storage failure, or missing candidate.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `arguments`: Stable candidate identity.
    /// - `cancelled`: Shared branch cancellation flag.
    /// - `super`: Parent module containing the agent tool adapter.
    ///
    /// Durable validation state and bounded diagnostics.
    ///
    /// Returns invalid input, stale-base, cancellation, or workspace failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn project_validate_candidate(
        &self,
        arguments: &serde_json::Value,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ToolResult, ToolError> {
        let id = candidate_id(arguments)?;
        let profile = arguments
            .get("profile")
            .and_then(serde_json::Value::as_str)
            .map_or(
                Some(CandidateValidationProfile::RustWorkspace),
                CandidateValidationProfile::from_identifier,
            )
            .ok_or_else(|| invalid_argument("unknown validation profile"))?;
        let candidate = self
            .project_service()?
            .validate_candidate_profile_cancellable(id, profile, cancelled)
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
                format!("entity:{}", candidate.entity),
                format!("path:{}", candidate.path),
                format!("validation:{}", validation_name(&candidate.validation)),
                format!("validation_id:{}:{}", candidate.id, profile.identifier()),
                format!("validation_profile:{}", profile.identifier()),
            ],
        );
        result.snapshot = Some(candidate.base_snapshot);
        result.object_type = Some("candidate_validation".to_owned());
        Ok(result)
    }

    /// Acquires dependencies fixed by a candidate lockfile in isolation.
    ///
    /// - `arguments`: Stable candidate identity.
    /// - `cancelled`: Shared branch cancellation flag.
    /// - `super`: Parent module containing the agent tool adapter.
    ///
    /// Durable dependency acquisition evidence bound to the candidate.
    ///
    /// Returns invalid input, stale-base, cancellation, or workspace failure.
    /// Acquisition failures are returned as durable failed evidence.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn project_resolve_dependencies(
        &self,
        arguments: &serde_json::Value,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ToolResult, ToolError> {
        let id = candidate_id(arguments)?;
        let candidate = self
            .project_service()?
            .resolve_candidate_dependencies_cancellable(id, cancelled)
            .map_err(|error| Self::project_error(&error))?;
        let (summary, resolution) = match &candidate.validation {
            CandidateValidation::Pending => (
                format!(
                    "locked dependency acquisition remains pending for candidate {}",
                    candidate.id
                ),
                "pending",
            ),
            CandidateValidation::Passed => (
                format!(
                    "locked dependency acquisition passed for candidate {}",
                    candidate.id
                ),
                "passed",
            ),
            CandidateValidation::Failed { diagnostics } => (
                format!(
                    "locked dependency acquisition failed for candidate {}: {}",
                    candidate.id,
                    diagnostics
                        .iter()
                        .take(8)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
                "failed",
            ),
        };
        let mut result = Self::result(
            summary,
            vec![
                format!("candidate:{}", candidate.id),
                format!("snapshot:{}", candidate.base_snapshot),
                format!("dependency_resolution:{resolution}"),
                format!("validation:{}", validation_name(&candidate.validation)),
                "validation_profile:cargo-dependency-resolution".to_owned(),
            ],
        );
        result.snapshot = Some(candidate.base_snapshot);
        result.object_type = Some("candidate_dependency_resolution".to_owned());
        Ok(result)
    }
}

/// Parses one project identity from provider arguments.
///
/// - `arguments`: Provider argument object.
///
/// Project identity.
///
/// Returns a structured invalid-argument failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
/// - `arguments`: Provider argument object.
///
/// Snapshot identity.
///
/// Returns a structured invalid-argument failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
/// - `value`: One structured effect object.
///
/// Shared project change request.
///
/// Returns invalid path, kind, or content input.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
/// - `arguments`: Provider argument object.
///
/// Candidate identifier text.
///
/// Returns a structured invalid-argument failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn candidate_id(arguments: &serde_json::Value) -> Result<&str, ToolError> {
    arguments
        .get("candidate")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid_argument("candidate"))
}

/// Requires a candidate operation to use the selected exact snapshot.
///
/// - `context`: Trusted operation context.
/// - `snapshot`: Snapshot requested by the provider.
///
/// Unit when the snapshot matches the user-selected project scope.
///
/// Returns a capability denial for an out-of-scope snapshot.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
/// - `kind`: Shared project effect kind.
///
/// Stable lowercase effect label.
/// # Returns
/// The operation result.
fn effect_kind(kind: ProjectSourceEffectKind) -> &'static str {
    match kind {
        ProjectSourceEffectKind::Create => "create",
        ProjectSourceEffectKind::Update => "update",
        ProjectSourceEffectKind::Delete => "delete",
    }
}

/// Names the durable candidate validation state.
///
/// - `validation`: Candidate validation record.
///
/// Stable concise state label.
/// # Returns
/// The operation result.
fn validation_name(validation: &CandidateValidation) -> &'static str {
    match validation {
        CandidateValidation::Pending => "pending",
        CandidateValidation::Passed => "passed",
        CandidateValidation::Failed { .. } => "failed",
    }
}
