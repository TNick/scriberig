//! Preflights and materializes aggregate project source effects.

use std::fs;
use std::path::{Path, PathBuf};

use sr_projection_fs::FileSystem;

use super::super::{
    ProjectCandidate, ProjectError, ProjectService, ProjectSourceEffect, ProjectSourceEffectKind,
    candidate_support,
};
use super::candidate_error;

/// Describes one preflighted materialization action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EffectAction {
    /// Create or replace exact candidate bytes.
    Write,
    /// Remove a base-owned source file.
    Delete,
    /// A prior durable receipt proves the intended state is already present.
    AlreadyApplied,
}

/// Carries a validated path and its candidate source effect.
pub(super) struct EffectPlan<'a> {
    /// Candidate-owned source effect.
    effect: &'a ProjectSourceEffect,
    /// Canonically contained filesystem target.
    path: PathBuf,
    /// Whether a prior durable receipt permits idempotent recovery.
    allow_already_applied: bool,
}

/// Preflights every aggregate path before materializing any effect.
///
/// - `service`: Project service used to persist path receipts.
/// - `candidate`: Durable aggregate candidate.
/// - `base_files`: Exact immutable source files at the candidate base.
/// - `root`: Authorized live project root.
///
/// Ordered per-path materialization plans.
///
/// Returns the first stale, ownership, path, or external filesystem conflict.
pub(super) fn preflight_effects<'a>(
    service: &ProjectService,
    candidate: &'a ProjectCandidate,
    base_files: &[scriberig_store_sqlite::ProjectFileRecord],
    root: &Path,
) -> Result<Vec<EffectPlan<'a>>, ProjectError> {
    let mut plans = Vec::with_capacity(candidate.effects.len());
    let prior = service
        .store
        .project_effect_paths(&candidate.id)
        .map_err(crate::errors::storage_error)?;
    for effect in &candidate.effects {
        let allow_already_applied = prior.iter().any(|receipt| {
            receipt.path == effect.path
                && matches!(receipt.state.as_str(), "applied" | "already_applied")
        });
        if !allow_already_applied {
            service.record_effect_path(candidate, effect, "not_attempted", None, None)?;
        }
        match inspect_effect(effect, base_files, root, allow_already_applied) {
            Ok((path, action, actual)) => {
                let state = if action == EffectAction::AlreadyApplied {
                    "already_applied"
                } else {
                    "preflighted"
                };
                service.record_effect_path(candidate, effect, state, actual.as_deref(), None)?;
                plans.push(EffectPlan {
                    effect,
                    path,
                    allow_already_applied,
                });
            }
            Err(error) => {
                service.record_effect_path(
                    candidate,
                    effect,
                    "conflict",
                    None,
                    Some(&error.message),
                )?;
                service.record_effect(candidate, None, "failed", Some(&error.message))?;
                return Err(error);
            }
        }
    }
    Ok(plans)
}

/// Rejects unrelated working-tree divergence before whole-project refresh.
///
/// - `base_files`: Exact canonical source manifest at the candidate base.
/// - `effects`: Candidate paths allowed to differ from the base.
/// - `root`: Authorized project checkout.
/// - `super`: Parent apply module visibility.
///
/// Unit when every unaffected scanned source matches the exact base.
///
/// Returns a path-specific conflict for unreviewed additions, edits, or deletions.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn preflight_project_tree(
    base_files: &[scriberig_store_sqlite::ProjectFileRecord],
    effects: &[ProjectSourceEffect],
    root: &Path,
) -> Result<(), ProjectError> {
    let changed = effects
        .iter()
        .map(|effect| effect.path.as_str())
        .collect::<std::collections::HashSet<_>>();
    let base = base_files
        .iter()
        .map(|file| (file.relative_path.as_str(), file))
        .collect::<std::collections::HashMap<_, _>>();
    let paths = super::super::apply_scope::candidate_scan_paths(root, base_files, effects)?;
    let current = crate::support::scan_directory(root, paths.as_ref())
        .map_err(|error| candidate_error("filesystem_scan_failed", &error.to_string()))?;
    let mut observed = std::collections::HashSet::new();
    for file in current.into_iter().filter(|file| file.status == "ready") {
        if changed.contains(file.relative_path.as_str()) {
            continue;
        }
        match base.get(file.relative_path.as_str()) {
            Some(base_file) if base_file.bytes == file.bytes => {
                observed.insert(file.relative_path);
            }
            _ => return Err(filesystem_conflict(&file.relative_path)),
        }
    }
    for file in base_files {
        if !changed.contains(file.relative_path.as_str())
            && !observed.contains(file.relative_path.as_str())
        {
            return Err(filesystem_conflict(&file.relative_path));
        }
    }
    Ok(())
}

/// Checks one effect against immutable base ownership and current disk state.
///
/// - `effect`: Candidate source effect.
/// - `base_files`: Exact base source manifest.
/// - `root`: Authorized project root.
/// - `allow_already_applied`: Whether a durable receipt permits recovery.
///
/// Safe path, required action, and currently observed bytes.
///
/// Returns an ownership, path, or external edit conflict.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn inspect_effect(
    effect: &ProjectSourceEffect,
    base_files: &[scriberig_store_sqlite::ProjectFileRecord],
    root: &Path,
    allow_already_applied: bool,
) -> Result<(PathBuf, EffectAction, Option<Vec<u8>>), ProjectError> {
    let path = safe_project_path(root, &effect.path)?;
    let base = base_files
        .iter()
        .find(|file| file.relative_path == effect.path);
    match (
        effect.kind,
        base,
        effect.old.as_deref(),
        effect.new.as_deref(),
    ) {
        (ProjectSourceEffectKind::Create, None, None, Some(new)) => {
            let actual = read_optional(&path)?;
            match actual.as_deref() {
                None => Ok((path, EffectAction::Write, None)),
                Some(bytes) if bytes == new && allow_already_applied => {
                    Ok((path, EffectAction::AlreadyApplied, actual))
                }
                Some(_) => Err(filesystem_conflict(&effect.path)),
            }
        }
        (ProjectSourceEffectKind::Update, Some(base), Some(old), Some(new))
            if base.bytes == old =>
        {
            let actual = read_optional(&path)?;
            match actual.as_deref() {
                Some(bytes) if bytes == old && bytes != new => {
                    Ok((path, EffectAction::Write, actual))
                }
                Some(bytes) if bytes == new && (allow_already_applied || old == new) => {
                    Ok((path, EffectAction::AlreadyApplied, actual))
                }
                _ => Err(filesystem_conflict(&effect.path)),
            }
        }
        (ProjectSourceEffectKind::Delete, Some(base), Some(old), None) if base.bytes == old => {
            let actual = read_optional(&path)?;
            match actual.as_deref() {
                Some(bytes) if bytes == old => Ok((path, EffectAction::Delete, actual)),
                None if allow_already_applied => Ok((path, EffectAction::AlreadyApplied, None)),
                _ => Err(filesystem_conflict(&effect.path)),
            }
        }
        _ => Err(candidate_error(
            "candidate_precondition_invalid",
            &effect.path,
        )),
    }
}

/// Executes a complete preflighted set and persists per-path progress.
///
/// - `service`: Project service storing durable progress.
/// - `candidate`: Candidate being materialized.
/// - `plans`: Full preflight result.
/// - `base_files`: File records used to verify the base state.
/// - `root`: Canonical project root on disk.
/// - `filesystem`: Filesystem adapter used for safe reads and writes.
/// - `super`: Parent module visibility.
///
/// Unit after every effect is materialized or already satisfied.
///
/// Returns a structured error and retains the completed path receipts.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn materialize_effects(
    service: &ProjectService,
    candidate: &ProjectCandidate,
    plans: &[EffectPlan<'_>],
    base_files: &[scriberig_store_sqlite::ProjectFileRecord],
    root: &Path,
    filesystem: &dyn FileSystem,
) -> Result<(), ProjectError> {
    for plan in plans {
        let (current_path, current_action, current) =
            match inspect_effect(plan.effect, base_files, root, plan.allow_already_applied) {
                Ok(value) => value,
                Err(error) => {
                    service.record_effect_path(
                        candidate,
                        plan.effect,
                        "conflict",
                        None,
                        Some(&error.message),
                    )?;
                    return Err(error);
                }
            };
        if current_path != plan.path {
            let error = candidate_error("unsafe_project_path", &plan.effect.path);
            service.record_effect_path(
                candidate,
                plan.effect,
                "failed",
                current.as_deref(),
                Some(&error.message),
            )?;
            return Err(error);
        }
        let result = match current_action {
            EffectAction::AlreadyApplied => Ok(current.clone()),
            EffectAction::Write => {
                let parent = plan
                    .path
                    .parent()
                    .ok_or_else(|| candidate_error("unsafe_project_path", &plan.effect.path))?;
                filesystem
                    .create_dir_all(parent)
                    .and_then(|()| {
                        filesystem.write_atomic(
                            &plan.path,
                            plan.effect.new.as_deref().unwrap_or_default(),
                        )
                    })
                    .map_err(|error| candidate_error("filesystem_write_failed", &error.to_string()))
                    .map(|()| plan.effect.new.clone())
            }
            EffectAction::Delete => filesystem
                .remove_file(&plan.path)
                .map_err(|error| candidate_error("filesystem_delete_failed", &error.to_string()))
                .map(|()| None),
        };
        match result {
            Ok(observed) => service.record_effect_path(
                candidate,
                plan.effect,
                "applied",
                observed.as_deref(),
                None,
            )?,
            Err(error) => {
                service.record_effect_path(
                    candidate,
                    plan.effect,
                    "failed",
                    current.as_deref(),
                    Some(&error.message),
                )?;
                return Err(error);
            }
        }
    }
    Ok(())
}

/// Resolves one project-relative target while rejecting traversal and symlinks.
///
/// - `root`: Authorized project root.
/// - `relative`: Project-relative target.
///
/// Canonically contained target path, including a missing final path.
///
/// Returns a structured path error for traversal, symlinks, or containment escape.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn safe_project_path(root: &Path, relative: &str) -> Result<PathBuf, ProjectError> {
    let parsed = Path::new(relative);
    if relative.is_empty()
        || relative.contains('\\')
        || parsed.is_absolute()
        || parsed
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(candidate_error("unsafe_project_path", relative));
    }
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| candidate_error("project_root_unavailable", &error.to_string()))?;
    let target = candidate_support::safe_join(&canonical_root, relative)?;
    let mut cursor = canonical_root.clone();
    for component in parsed.components() {
        let std::path::Component::Normal(name) = component else {
            return Err(candidate_error("unsafe_project_path", relative));
        };
        cursor.push(name);
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(candidate_error("unsafe_project_path", relative));
            }
            Ok(_) => {
                let canonical = fs::canonicalize(&cursor)
                    .map_err(|error| candidate_error("unsafe_project_path", &error.to_string()))?;
                if !canonical.starts_with(&canonical_root) {
                    return Err(candidate_error("unsafe_project_path", relative));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => {
                return Err(candidate_error(
                    "filesystem_metadata_failed",
                    &error.to_string(),
                ));
            }
        }
    }
    Ok(target)
}

/// Reads a target when present while preserving non-not-found IO errors.
///
/// - `path`: Safe project target path.
///
/// Exact current bytes or `None` when the target is absent.
///
/// Returns a filesystem read error for inaccessible targets.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, ProjectError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(candidate_error(
            "filesystem_read_failed",
            &error.to_string(),
        )),
    }
}

/// Creates a path-specific external modification conflict.
///
/// - `path`: Conflicting project-relative path.
///
/// Structured filesystem conflict.
/// # Returns
/// The operation result.
fn filesystem_conflict(path: &str) -> ProjectError {
    candidate_error(
        "filesystem_conflict",
        &format!("source changed externally: {path}"),
    )
}
