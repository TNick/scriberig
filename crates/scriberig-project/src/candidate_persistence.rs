//! Converts aggregate source candidates to and from durable storage records.

use scriberig_store_sqlite::{ProjectCandidateRecord, ProjectSourceEffectRecord};

use super::{
    CandidateValidation, ProjectCandidate, ProjectError, ProjectSourceEffect,
    ProjectSourceEffectKind,
};

impl ProjectCandidate {
    /// Converts a project candidate into durable storage data.
    ///
    /// Storage-neutral candidate fields.
    ///
    /// - `super`: Candidate module visibility.
    /// # Returns
    /// The operation result.
    pub(super) fn record(&self) -> ProjectCandidateRecord {
        let (validation, diagnostics) = match &self.validation {
            CandidateValidation::Pending => ("pending", String::new()),
            CandidateValidation::Passed => ("passed", String::new()),
            CandidateValidation::Failed { diagnostics } => ("failed", diagnostics.join("\n")),
        };
        ProjectCandidateRecord {
            id: self.id.clone(),
            project: self.project,
            base_snapshot: self.base_snapshot,
            entity: self.entity,
            path: self.path.clone(),
            old: self.old.clone(),
            new: self.new.clone(),
            validation: validation.to_owned(),
            diagnostics,
            effects: self
                .effects
                .iter()
                .map(|effect| ProjectSourceEffectRecord {
                    path: effect.path.clone(),
                    kind: match effect.kind {
                        ProjectSourceEffectKind::Create => "create",
                        ProjectSourceEffectKind::Update => "update",
                        ProjectSourceEffectKind::Delete => "delete",
                    }
                    .to_owned(),
                    old: effect.old.clone(),
                    new: effect.new.clone(),
                })
                .collect(),
            validation_profile: self.validation_profile.clone(),
        }
    }

    /// Reconstructs a candidate loaded from durable storage.
    ///
    /// - `record`: Durable candidate record.
    /// - `super`: Candidate module visibility.
    ///
    /// The public project candidate.
    ///
    /// Returns a project error when persisted validation state is invalid.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn from_record(record: ProjectCandidateRecord) -> Result<Self, ProjectError> {
        let validation = match record.validation.as_str() {
            "pending" => CandidateValidation::Pending,
            "passed" => CandidateValidation::Passed,
            "failed" => CandidateValidation::Failed {
                diagnostics: record.diagnostics.lines().map(ToOwned::to_owned).collect(),
            },
            _ => {
                return Err(ProjectError {
                    code: "candidate_corrupt",
                    message: "unknown durable candidate validation state".to_owned(),
                    diagnostics: Vec::new(),
                });
            }
        };
        let effects = record
            .effects
            .into_iter()
            .map(|effect| {
                let kind = match effect.kind.as_str() {
                    "create" => ProjectSourceEffectKind::Create,
                    "update" => ProjectSourceEffectKind::Update,
                    "delete" => ProjectSourceEffectKind::Delete,
                    _ => {
                        return Err(ProjectError {
                            code: "candidate_corrupt",
                            message: "unknown source effect kind".to_owned(),
                            diagnostics: Vec::new(),
                        });
                    }
                };
                Ok(ProjectSourceEffect {
                    path: effect.path,
                    kind,
                    old: effect.old,
                    new: effect.new,
                })
            })
            .collect::<Result<Vec<_>, ProjectError>>()?;
        let effects = if effects.is_empty() {
            vec![ProjectSourceEffect {
                path: record.path.clone(),
                kind: ProjectSourceEffectKind::Update,
                old: Some(record.old.clone()),
                new: Some(record.new.clone()),
            }]
        } else {
            effects
        };
        Ok(Self {
            id: record.id,
            project: record.project,
            base_snapshot: record.base_snapshot,
            entity: record.entity,
            path: record.path,
            old: record.old,
            new: record.new,
            effects,
            validation,
            validation_profile: record.validation_profile,
        })
    }
}
