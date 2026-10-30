//! Exposes authorized QGIS layer-reference rename candidates.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_project::CandidateValidation;

use crate::{
    ApplicationContext, ApplicationError, ApplicationService, CapabilityAction, CapabilityScope,
    OperationKind,
};

/// Contains a durable QGIS layer-reference rename candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QgisLayerRenameCandidate {
    /// Stable project candidate identity.
    pub id: String,
    /// Project owning the QGIS project source.
    pub project: ProjectId,
    /// Exact snapshot against which the rename was proposed.
    pub base_snapshot: SnapshotId,
    /// Stable semantic layer-reference identity.
    pub reference: EntityId,
    /// Exact project-relative QGS/QGZ path.
    pub path: String,
    /// Current candidate validation state.
    pub validation: CandidateValidation,
}

/// Contains a durable QGIS project layer-order candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QgisLayerOrderCandidate {
    /// Stable project candidate identity.
    pub id: String,
    /// Project owning the QGIS project source.
    pub project: ProjectId,
    /// Exact snapshot against which the order was proposed.
    pub base_snapshot: SnapshotId,
    /// Exact project-relative QGS/QGZ path.
    pub path: String,
    /// Current candidate validation state.
    pub validation: CandidateValidation,
}

impl<B> ApplicationService<B> {
    /// Proposes a complete QGIS layer order under exact-snapshot project authority.
    ///
    /// # Parameters
    /// - `context`: Request context with exact snapshot and project-scoped Propose grant.
    /// - `project`: Project owning the QGIS source.
    /// - `path`: Exact project-relative QGS/QGZ path.
    /// - `ordered_references`: Complete ordered semantic layer-reference identities.
    ///
    /// # Returns
    /// Candidate metadata for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns authorization, request, unsupported-operation, or candidate errors.
    pub fn propose_qgis_layer_order(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        path: &str,
        ordered_references: &[EntityId],
    ) -> Result<QgisLayerOrderCandidate, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "qgis.layer_order.reorder".into(),
                message: "QGIS edits require an exact snapshot".into(),
            })?;
        let project_scope = CapabilityScope::Custom(format!("project:{project}"));
        if !context.capabilities.permits(
            &CapabilityAction::Propose,
            &project_scope,
            Some(snapshot),
            Some(OperationKind::DomainOperation),
        ) {
            return Err(ApplicationError::PermissionDenied {
                action: "Propose".into(),
                scope: format!("QGIS project {project}"),
            });
        }
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "qgis.layer_order.reorder".into(),
            }
        })?;
        let candidate = service
            .propose_qgis_layer_order(project, snapshot, path, ordered_references)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "qgis.layer_order.reorder".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(QgisLayerOrderCandidate {
            id: candidate.id,
            project: candidate.project,
            base_snapshot: candidate.base_snapshot,
            path: candidate.path,
            validation: candidate.validation,
        })
    }

    /// Proposes one QGIS layer display-name update under exact-snapshot authority.
    ///
    /// # Parameters
    /// - `context`: Request context with exact snapshot and scoped Propose grant.
    /// - `project`: Project owning the QGIS project source.
    /// - `reference`: Stable semantic QGIS layer-reference identity.
    /// - `new_name`: New authored display name.
    ///
    /// # Returns
    /// Candidate metadata for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns authorization, request, unsupported-operation, or candidate errors.
    pub fn propose_qgis_layer_rename(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        reference: EntityId,
        new_name: &str,
    ) -> Result<QgisLayerRenameCandidate, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "qgis.layer.rename".into(),
                message: "QGIS edits require an exact snapshot".into(),
            })?;
        let target_scope = CapabilityScope::Entity(reference);
        let project_scope = CapabilityScope::Custom(format!("project:{project}"));
        let operation = Some(OperationKind::DomainOperation);
        if !context.capabilities.permits(
            &CapabilityAction::Propose,
            &target_scope,
            Some(snapshot),
            operation,
        ) && !context.capabilities.permits(
            &CapabilityAction::Propose,
            &project_scope,
            Some(snapshot),
            operation,
        ) {
            return Err(ApplicationError::PermissionDenied {
                action: "Propose".into(),
                scope: format!("QGIS layer reference {reference} in project {project}"),
            });
        }
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "qgis.layer.rename".into(),
            }
        })?;
        let candidate = service
            .propose_qgis_layer_rename(project, snapshot, reference, new_name)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "qgis.layer.rename".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(QgisLayerRenameCandidate {
            id: candidate.id,
            project: candidate.project,
            base_snapshot: candidate.base_snapshot,
            reference,
            path: candidate.path,
            validation: candidate.validation,
        })
    }
}
