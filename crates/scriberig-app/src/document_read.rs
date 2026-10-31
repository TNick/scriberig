//! Exposes authorized snapshot-bound rich-document semantic views.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_project::{DocumentParagraphSearch, DocumentTreeView};

use crate::{
    ApplicationContext, ApplicationError, ApplicationService, CapabilityAction, CapabilityScope,
    OperationKind,
};

/// Identifies the exact document snapshot and page used for paragraph search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentParagraphSearchRequest {
    /// Project that owns the document.
    pub project: ProjectId,
    /// Exact immutable snapshot to search.
    pub snapshot: SnapshotId,
    /// Stable semantic document root.
    pub document: EntityId,
    /// Non-empty text substring to find.
    pub query: String,
    /// Zero-based result offset.
    pub offset: usize,
    /// Maximum number of matches to return.
    pub limit: usize,
}

impl<B> ApplicationService<B> {
    /// Finds paragraph text through project-scoped Inspect authority.
    ///
    /// # Parameters
    /// - `context`: Caller context bound to the exact requested snapshot.
    /// - `request`: Project, exact snapshot, document, query, and page bounds.
    ///
    /// # Returns
    /// Bounded paragraph matches with stable semantic identities.
    ///
    /// # Errors
    /// Returns authorization, availability, stale-snapshot, or project search errors.
    pub fn find_document_paragraphs(
        &self,
        context: &ApplicationContext,
        request: &DocumentParagraphSearchRequest,
    ) -> Result<DocumentParagraphSearch, ApplicationError> {
        let DocumentParagraphSearchRequest {
            project,
            snapshot,
            document,
            query,
            offset,
            limit,
        } = request;
        let (project, snapshot, document) = (*project, *snapshot, *document);
        if context.snapshot != Some(snapshot) {
            return Err(ApplicationError::InvalidRequest {
                operation: "document.paragraph.find".to_owned(),
                message: "document search requires the exact requested snapshot".into(),
            });
        }
        let scope = CapabilityScope::Custom(format!("project:{project}"));
        if !context.capabilities.permits(
            &CapabilityAction::Inspect,
            &scope,
            Some(snapshot),
            Some(OperationKind::Inspect),
        ) {
            return Err(ApplicationError::PermissionDenied {
                action: "Inspect".to_owned(),
                scope: format!("project:{project}"),
            });
        }
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "document.paragraph.find".to_owned(),
            }
        })?;
        service
            .find_document_paragraphs(project, snapshot, document, query, *offset, *limit)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "document.paragraph.find".to_owned(),
                message: format!("{}: {}", error.code, error.message),
            })
    }

    /// Reads a bounded semantic document tree through project-scoped Inspect authority.
    ///
    /// # Parameters
    /// - `context`: Caller capability context bound to the exact snapshot.
    /// - `project`: Project owning the document.
    /// - `snapshot`: Exact immutable snapshot requested by the caller.
    /// - `document`: Stable semantic document root identity.
    ///
    /// # Returns
    /// Typed ordered document sections, paragraphs, and tables.
    ///
    /// # Errors
    /// Returns authorization, availability, stale-snapshot, or project read errors.
    pub fn inspect_document(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        snapshot: SnapshotId,
        document: EntityId,
    ) -> Result<DocumentTreeView, ApplicationError> {
        if context.snapshot != Some(snapshot) {
            return Err(ApplicationError::InvalidRequest {
                operation: "document.inspect".to_owned(),
                message: "document inspection requires the exact requested snapshot".to_owned(),
            });
        }
        let scope = CapabilityScope::Custom(format!("project:{project}"));
        if !context.capabilities.permits(
            &CapabilityAction::Inspect,
            &scope,
            Some(snapshot),
            Some(OperationKind::Inspect),
        ) {
            return Err(ApplicationError::PermissionDenied {
                action: "Inspect".to_owned(),
                scope: format!("project:{project}"),
            });
        }
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "document.inspect".to_owned(),
            }
        })?;
        service
            .document_tree(project, snapshot, document)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "document.inspect".to_owned(),
                message: format!("{}: {}", error.code, error.message),
            })
    }
}
