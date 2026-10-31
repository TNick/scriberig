//! Exposes bounded artifact-member evidence through shared Inspect authority.

use scriberig_core::{EntityId, ProjectId};
use scriberig_project::{ArtifactMemberDescriptor, ArtifactMemberPage as ProjectMemberPage};

use crate::{
    ApplicationContext, ApplicationError, ApplicationService, CapabilityAction, CapabilityScope,
    OperationKind,
};

/// Maximum member descriptors returned by one artifact inspection request.
pub const MAX_ARTIFACT_MEMBER_PAGE: usize = 500;

/// Contains a bounded page of exact artifact-member evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactMembers {
    /// Stable project identity owning the artifact.
    pub project: ProjectId,
    /// Stable artifact identity whose members are returned.
    pub artifact: EntityId,
    /// Exact physical and container-internal descriptors for this page.
    pub members: Vec<ArtifactMemberDescriptor>,
    /// Total member count in the exact artifact manifest.
    pub total: usize,
    /// Offset for the next page, when present.
    pub next_offset: Option<usize>,
}

impl<B> ApplicationService<B> {
    /// Reads one bounded artifact-member page under project Inspect authority.
    ///
    /// # Parameters
    /// - `context`: Authorized request context.
    /// - `project`: Project owning the artifact.
    /// - `artifact`: Stable semantic artifact identity.
    /// - `offset`: Zero-based member offset.
    /// - `limit`: Maximum member count, from one through five hundred.
    ///
    /// # Returns
    /// Exact member descriptors with deterministic continuation metadata.
    ///
    /// # Errors
    /// Returns authorization, request, catalog, or availability errors.
    pub fn inspect_artifact_members(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        artifact: EntityId,
        offset: usize,
        limit: usize,
    ) -> Result<ArtifactMembers, ApplicationError> {
        if !(1..=MAX_ARTIFACT_MEMBER_PAGE).contains(&limit) {
            return Err(ApplicationError::InvalidRequest {
                operation: "artifact.list_members".into(),
                message: "artifact member page limit must be between one and five hundred".into(),
            });
        }
        let scope = CapabilityScope::Custom(format!("project:{project}"));
        if !context.capabilities.permits(
            &CapabilityAction::Inspect,
            &scope,
            context.snapshot,
            Some(OperationKind::Inspect),
        ) {
            return Err(ApplicationError::PermissionDenied {
                action: "Inspect".into(),
                scope: format!("project:{project}"),
            });
        }
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "artifact.list_members".into(),
            }
        })?;
        let ProjectMemberPage {
            members,
            total,
            next_offset,
            ..
        } = service
            .artifact_members(project, artifact, offset, limit)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "artifact.list_members".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(ArtifactMembers {
            project,
            artifact,
            members,
            total,
            next_offset,
        })
    }
}
