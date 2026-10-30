//! Exposes authorized DOCX paragraph candidate creation to product surfaces.

use scriberig_core::{EntityId, ProjectId};
use scriberig_project::CandidateValidation;

use crate::{
    ApplicationContext, ApplicationError, ApplicationService, CapabilityAction, CapabilityScope,
    OperationKind,
};

/// Contains a durable DOCX paragraph candidate without exposing package bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentParagraphCandidate {
    /// Stable project candidate identity.
    pub id: String,
    /// Project owning the candidate.
    pub project: ProjectId,
    /// Exact snapshot against which the edit was proposed.
    pub base_snapshot: scriberig_core::SnapshotId,
    /// Project-relative DOCX path.
    pub path: String,
    /// Current validation state of the candidate.
    pub validation: CandidateValidation,
}

impl<B> ApplicationService<B> {
    /// Proposes a semantic DOCX table-cell edit through its stable cell handle.
    ///
    /// # Parameters
    /// - `context`: Request context with an exact snapshot and scoped Propose grant.
    /// - `project`: Project owning the DOCX source.
    /// - `cell`: Stable semantic table-cell identity.
    /// - `replacement`: Replacement UTF-8 cell text.
    ///
    /// # Returns
    /// Candidate metadata for shared validation and review.
    ///
    /// # Errors
    /// Returns authorization, stale-base, unsupported-cell-structure, or candidate errors.
    pub fn propose_document_table_cell_edit(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        cell: EntityId,
        replacement: &str,
    ) -> Result<DocumentParagraphCandidate, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "document.table_cell.propose".into(),
                message: "document edits require an exact snapshot".into(),
            })?;
        authorize_document_proposal(context, project, cell, snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "document.table_cell.propose".into(),
            }
        })?;
        let candidate = service
            .propose_docx_table_cell_edit(project, snapshot, cell, replacement)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "document.table_cell.propose".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(DocumentParagraphCandidate {
            id: candidate.id,
            project: candidate.project,
            base_snapshot: candidate.base_snapshot,
            path: candidate.path,
            validation: candidate.validation,
        })
    }

    /// Proposes a semantic DOCX paragraph append as a durable candidate.
    ///
    /// # Parameters
    /// - `context`: Request context with an exact snapshot and scoped Propose grant.
    /// - `project`: Project owning the DOCX source.
    /// - `section`: Stable semantic section identity receiving the new paragraph.
    /// - `text`: New UTF-8 paragraph text.
    ///
    /// # Returns
    /// Candidate metadata for shared validation and review.
    ///
    /// # Errors
    /// Returns authorization, stale-base, unsupported-operation, or candidate errors.
    pub fn propose_document_paragraph_insert(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        section: EntityId,
        text: &str,
    ) -> Result<DocumentParagraphCandidate, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "document.paragraph.insert".into(),
                message: "document edits require an exact snapshot".into(),
            })?;
        authorize_document_proposal(context, project, section, snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "document.paragraph.insert".into(),
            }
        })?;
        let candidate = service
            .propose_docx_paragraph_insert(project, snapshot, section, text)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "document.paragraph.insert".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(DocumentParagraphCandidate {
            id: candidate.id,
            project: candidate.project,
            base_snapshot: candidate.base_snapshot,
            path: candidate.path,
            validation: candidate.validation,
        })
    }

    /// Proposes a semantic DOCX paragraph edit as a durable candidate.
    ///
    /// # Parameters
    /// - `context`: Request context with an exact snapshot and scoped Propose grant.
    /// - `project`: Project owning the DOCX source.
    /// - `paragraph`: Stable semantic paragraph identity.
    /// - `replacement`: Replacement UTF-8 paragraph text.
    ///
    /// # Returns
    /// Candidate metadata for shared validation and review.
    ///
    /// # Errors
    /// Returns authorization, unsupported-operation, or candidate errors.
    pub fn propose_document_paragraph_edit(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        paragraph: EntityId,
        replacement: &str,
    ) -> Result<DocumentParagraphCandidate, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "document.paragraph.propose".into(),
                message: "document edits require an exact snapshot".into(),
            })?;
        authorize_document_proposal(context, project, paragraph, snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "document.paragraph.propose".into(),
            }
        })?;
        let candidate = service
            .propose_docx_paragraph_edit(project, snapshot, paragraph, replacement)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "document.paragraph.propose".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(DocumentParagraphCandidate {
            id: candidate.id,
            project: candidate.project,
            base_snapshot: candidate.base_snapshot,
            path: candidate.path,
            validation: candidate.validation,
        })
    }
}

/// Requires exact-target or explicit project-wide document proposal authority.
///
/// # Parameters
/// - `context`: Request capability grants.
/// - `project`: Project owning the target.
/// - `target`: Exact paragraph, cell, or section being edited.
/// - `snapshot`: Immutable base snapshot for the proposal.
///
/// # Returns
/// Unit when one supported Propose scope authorizes the exact request.
///
/// # Errors
/// Returns a permission error when no exact or project-wide grant applies.
fn authorize_document_proposal(
    context: &ApplicationContext,
    project: ProjectId,
    target: EntityId,
    snapshot: scriberig_core::SnapshotId,
) -> Result<(), ApplicationError> {
    let operation = Some(OperationKind::DomainOperation);
    let target_scope = CapabilityScope::Entity(target);
    let project_scope = CapabilityScope::Custom(format!("project:{project}"));
    if context.capabilities.permits(
        &CapabilityAction::Propose,
        &target_scope,
        Some(snapshot),
        operation,
    ) || context.capabilities.permits(
        &CapabilityAction::Propose,
        &project_scope,
        Some(snapshot),
        operation,
    ) {
        return Ok(());
    }
    Err(ApplicationError::PermissionDenied {
        action: "Propose".into(),
        scope: format!("document target {target} in project {project}"),
    })
}
