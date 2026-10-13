//! Builds durable aggregate project source candidates.

use scriberig_core::{ProjectId, SnapshotId};

use super::candidate_support::candidate_id;
use super::{
    CandidateValidation, ProjectCandidate, ProjectError, ProjectService, ProjectSourceChange,
    ProjectSourceChangeKind, ProjectSourceEffect, ProjectSourceEffectKind,
};

/// Validates a project-relative path without consulting the filesystem.
///
/// # Parameters
/// - `path`: Client-provided project-relative path.
///
/// # Returns
/// Unit for a normalized relative path.
///
/// # Errors
/// Returns a structured error for empty, absolute, traversal, or non-normal paths.
fn validate_relative_path(path: &str) -> Result<(), ProjectError> {
    let parsed = std::path::Path::new(path);
    if path.is_empty()
        || path.len() > 4096
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || parsed.is_absolute()
        || parsed
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(candidate_error("unsafe_project_path", path));
    }
    Ok(())
}

/// Builds a concise candidate request error.
///
/// # Parameters
/// - `code`: Stable error category.
/// - `message`: Safe diagnostic detail.
///
/// # Returns
/// Structured project error.
fn candidate_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}

/// Resolves one client change against its exact project file manifest.
///
/// # Parameters
/// - `change`: Requested source effect.
/// - `files`: Source files owned by the candidate base.
/// - `previous_path`: Prior sorted path used to reject duplicates.
///
/// # Returns
/// Effect enriched with exact base source bytes where required.
///
/// # Errors
/// Returns a structured ownership, path, duplicate, or size error.
fn resolve_effect(
    change: ProjectSourceChange,
    files: &[scriberig_store_sqlite::ProjectFileRecord],
    previous_path: Option<&str>,
) -> Result<ProjectSourceEffect, ProjectError> {
    validate_relative_path(&change.path)?;
    if previous_path == Some(change.path.as_str()) {
        return Err(candidate_error("duplicate_effect_path", &change.path));
    }
    if change
        .content
        .as_ref()
        .is_some_and(|content| content.len() > 2 * 1024 * 1024)
    {
        return Err(candidate_error("source_effect_too_large", &change.path));
    }
    let base = files.iter().find(|file| file.relative_path == change.path);
    let (kind, old, new) = match (change.kind, base, change.content) {
        (ProjectSourceChangeKind::Create, None, Some(content)) => {
            (ProjectSourceEffectKind::Create, None, Some(content))
        }
        (ProjectSourceChangeKind::Create, Some(_), _) => {
            return Err(candidate_error("path_exists", &change.path));
        }
        (ProjectSourceChangeKind::Update, Some(file), Some(content)) => (
            ProjectSourceEffectKind::Update,
            Some(file.bytes.clone()),
            Some(content),
        ),
        (ProjectSourceChangeKind::Update | ProjectSourceChangeKind::Delete, None, _) => {
            return Err(candidate_error("source_not_owned", &change.path));
        }
        (ProjectSourceChangeKind::Delete, Some(file), None) => (
            ProjectSourceEffectKind::Delete,
            Some(file.bytes.clone()),
            None,
        ),
        _ => return Err(candidate_error("invalid_effect", &change.path)),
    };
    Ok(ProjectSourceEffect {
        path: change.path,
        kind,
        old,
        new,
    })
}

impl ProjectService {
    /// Creates one durable exact-base candidate containing ordered path effects.
    ///
    /// # Parameters
    /// - `project`: Project receiving the candidate.
    /// - `base_snapshot`: Exact active snapshot required by the request.
    /// - `changes`: Ordered create, update, or delete requests.
    ///
    /// # Returns
    /// One candidate whose effects are persisted together without changing disk.
    ///
    /// # Errors
    /// Returns an error for stale bases, conflicting or invalid paths, ambiguous
    /// effects, and source ownership mismatches.
    pub fn propose_project_change(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        changes: Vec<ProjectSourceChange>,
    ) -> Result<ProjectCandidate, ProjectError> {
        if changes.is_empty() || changes.len() > 512 {
            return Err(candidate_error(
                "invalid_effect_count",
                "candidate must contain 1 to 512 effects",
            ));
        }
        let workspace = self.workspace(project)?;
        if workspace.snapshot != base_snapshot {
            return Err(candidate_error(
                "stale_candidate",
                "requested base is not the active project snapshot",
            ));
        }
        let files = self
            .store
            .project_files(project)
            .map_err(crate::errors::storage_error)?;
        let mut changes = changes;
        changes.sort_by(|left, right| left.path.cmp(&right.path));
        let total_bytes = changes.iter().try_fold(0_usize, |total, change| {
            total.checked_add(change.content.as_ref().map_or(0, Vec::len))
        });
        if total_bytes.is_none_or(|total| total > 8 * 1024 * 1024) {
            return Err(candidate_error(
                "candidate_too_large",
                "aggregate source effects exceed 8 MiB",
            ));
        }
        let mut effects = Vec::with_capacity(changes.len());
        for change in changes {
            let previous_path = effects
                .last()
                .map(|effect: &ProjectSourceEffect| effect.path.as_str());
            effects.push(resolve_effect(change, &files, previous_path)?);
        }
        let mut hasher = blake3::Hasher::new();
        for effect in &effects {
            hasher.update(effect.path.as_bytes());
            hasher.update(format!("{:?}", effect.kind).as_bytes());
            if let Some(old) = &effect.old {
                hasher.update(old);
            }
            if let Some(new) = &effect.new {
                hasher.update(new);
            }
        }
        let digest = hasher.finalize().to_hex().to_string();
        let id = candidate_id(project, base_snapshot, workspace.root_entity, &digest);
        let first = effects.first().ok_or_else(|| {
            candidate_error("invalid_effect_count", "candidate effects are empty")
        })?;
        let candidate = ProjectCandidate {
            id,
            project,
            base_snapshot,
            entity: workspace.root_entity,
            path: first.path.clone(),
            old: first.old.clone().unwrap_or_default(),
            new: first.new.clone().unwrap_or_default(),
            effects,
            validation: CandidateValidation::Pending,
        };
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)?;
        Ok(candidate)
    }
}
