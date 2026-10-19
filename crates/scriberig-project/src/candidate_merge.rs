//! Merges isolated project candidates without promoting project state.

use std::collections::BTreeMap;

use super::{
    CandidateValidation, ProjectCandidate, ProjectError, ProjectService, ProjectSourceChange,
    ProjectSourceChangeKind,
};

impl ProjectService {
    /// Merges two exact-base source candidates into a new durable candidate.
    ///
    /// The merge combines disjoint path effects. Conflicting edits to one path
    /// return a structured conflict and never mutate the project workspace.
    ///
    /// - `left_id`: Candidate handle produced by one completed dependency.
    /// - `right_id`: Candidate handle produced by another completed dependency.
    ///
    /// A new candidate that still requires normal validation and application.
    ///
    /// Returns a structured error for missing, incompatible, failed, or
    /// conflicting candidates.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the merge cannot be represented safely.
    pub fn merge_project_candidates(
        &self,
        left_id: &str,
        right_id: &str,
    ) -> Result<ProjectCandidate, ProjectError> {
        let left = self
            .candidate(left_id)?
            .ok_or_else(|| candidate_error("left candidate is missing"))?;
        let right = self
            .candidate(right_id)?
            .ok_or_else(|| candidate_error("right candidate is missing"))?;
        if left.id == right.id {
            return Err(candidate_error("candidate merge requires distinct inputs"));
        }
        if left.project != right.project || left.base_snapshot != right.base_snapshot {
            return Err(candidate_error(
                "candidate inputs must share one project and exact base snapshot",
            ));
        }
        if matches!(left.validation, CandidateValidation::Failed { .. })
            || matches!(right.validation, CandidateValidation::Failed { .. })
        {
            return Err(candidate_error("failed candidates cannot be integrated"));
        }

        let mut effects = BTreeMap::new();
        for effect in left.effects.iter().chain(&right.effects) {
            if let Some(existing) = effects.get(&effect.path) {
                if existing != effect {
                    return Err(ProjectError {
                        code: "candidate_merge_conflict",
                        message: format!("candidate effects conflict at {}", effect.path),
                        diagnostics: Vec::new(),
                    });
                }
            } else {
                effects.insert(effect.path.clone(), effect.clone());
            }
        }
        let changes = effects
            .into_values()
            .map(|effect| ProjectSourceChange {
                path: effect.path,
                kind: match effect.kind {
                    super::ProjectSourceEffectKind::Create => ProjectSourceChangeKind::Create,
                    super::ProjectSourceEffectKind::Update => ProjectSourceChangeKind::Update,
                    super::ProjectSourceEffectKind::Delete => ProjectSourceChangeKind::Delete,
                },
                content: effect.new,
            })
            .collect();
        self.propose_project_change(left.project, left.base_snapshot, changes)
    }
}

/// Creates a stable structured candidate merge error.
///
/// - `message`: Safe merge failure detail.
///
/// Candidate merge error without external diagnostics.
/// # Returns
/// The operation result.
fn candidate_error(message: &str) -> ProjectError {
    ProjectError {
        code: "candidate_merge_invalid",
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
