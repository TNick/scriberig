//! Exposes authorized semantic CAD edit candidates through the app service.

use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_cad::Coordinate;
use scriberig_project::CandidateValidation;

use crate::{
    ApplicationContext, ApplicationError, ApplicationService, CapabilityAction, CapabilityScope,
    OperationKind,
};

/// Contains a durable DXF line candidate without exposing package bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadLineCandidate {
    /// Stable project candidate identity.
    pub id: String,
    /// Project owning the source drawing.
    pub project: ProjectId,
    /// Exact snapshot against which the operation was proposed.
    pub base_snapshot: scriberig_core::SnapshotId,
    /// Project-relative DXF source path.
    pub path: String,
    /// Current candidate validation state.
    pub validation: CandidateValidation,
}

/// Contains a durable DXF layer-change candidate without exposing package bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadLayerCandidate {
    /// Stable project candidate identity.
    pub id: String,
    /// Project owning the source drawing.
    pub project: ProjectId,
    /// Exact snapshot against which the operation was proposed.
    pub base_snapshot: scriberig_core::SnapshotId,
    /// Project-relative DXF source path.
    pub path: String,
    /// Current candidate validation state.
    pub validation: CandidateValidation,
}

impl<B> ApplicationService<B> {
    /// Reads a snapshot-bound CAD drawing using project-scoped Inspect authority.
    ///
    /// # Parameters
    /// - `context`: Caller context bound to the exact requested snapshot.
    /// - `project`: Project owning the drawing.
    /// - `snapshot`: Exact immutable snapshot to inspect.
    /// - `drawing`: Stable semantic drawing identity.
    ///
    /// # Returns
    /// Typed drawing summary and its semantic layers.
    ///
    /// # Errors
    /// Returns an authorization, unsupported-operation, or project read error.
    pub fn inspect_cad_drawing(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        snapshot: scriberig_core::SnapshotId,
        drawing: EntityId,
    ) -> Result<scriberig_project::CadDrawingView, ApplicationError> {
        if context.snapshot != Some(snapshot) {
            return Err(ApplicationError::InvalidRequest {
                operation: "cad.drawing.inspect".into(),
                message: "CAD inspection requires the exact requested snapshot".into(),
            });
        }
        authorize_cad_inspect(context, project, snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "cad.drawing.inspect".into(),
            }
        })?;
        service
            .cad_drawing(project, snapshot, drawing)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "cad.drawing.inspect".into(),
                message: format!("{}: {}", error.code, error.message),
            })
    }

    /// Reads one bounded snapshot-bound CAD entity page using Inspect authority.
    ///
    /// # Parameters
    /// - `context`: Caller context bound to the exact requested snapshot.
    /// - `project`: Project owning the drawing.
    /// - `snapshot`: Exact immutable snapshot to inspect.
    /// - `drawing`: Stable semantic drawing identity.
    /// - `offset`: Zero-based offset in drawing source order.
    /// - `limit`: Requested page size from one through five hundred.
    ///
    /// # Returns
    /// Typed semantic entities with page continuation metadata.
    ///
    /// # Errors
    /// Returns an authorization, unsupported-operation, or project read error.
    pub fn read_cad_entities(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        snapshot: scriberig_core::SnapshotId,
        drawing: EntityId,
        offset: usize,
        limit: usize,
    ) -> Result<scriberig_project::CadEntityPage, ApplicationError> {
        if context.snapshot != Some(snapshot) {
            return Err(ApplicationError::InvalidRequest {
                operation: "cad.entities.read".into(),
                message: "CAD reads require the exact requested snapshot".into(),
            });
        }
        authorize_cad_inspect(context, project, snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "cad.entities.read".into(),
            }
        })?;
        service
            .cad_entities(project, snapshot, drawing, offset, limit)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "cad.entities.read".into(),
                message: format!("{}: {}", error.code, error.message),
            })
    }

    /// Proposes exact line geometry through scoped authorization and project candidates.
    ///
    /// # Parameters
    /// - `context`: Request context with an exact snapshot and Propose grant.
    /// - `project`: Project owning the drawing.
    /// - `entity`: Stable semantic line identity.
    /// - `start`: New exact decimal start coordinate.
    /// - `end`: New exact decimal end coordinate.
    ///
    /// # Returns
    /// Candidate metadata for shared review, validation, and apply.
    ///
    /// # Errors
    /// Returns an authorization, unsupported-operation, or candidate error.
    pub fn propose_cad_line_geometry(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        entity: EntityId,
        start: &Coordinate,
        end: &Coordinate,
    ) -> Result<CadLineCandidate, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "cad.line.propose".into(),
                message: "CAD edits require an exact snapshot".into(),
            })?;
        authorize_cad_proposal(context, project, entity, snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "cad.line.propose".into(),
            }
        })?;
        let candidate = service
            .propose_dxf_line_geometry(project, snapshot, entity, start, end)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "cad.line.propose".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(CadLineCandidate {
            id: candidate.id,
            project: candidate.project,
            base_snapshot: candidate.base_snapshot,
            path: candidate.path,
            validation: candidate.validation,
        })
    }

    /// Proposes moving one CAD entity to an existing layer in the exact drawing.
    ///
    /// # Parameters
    /// - `context`: Request context with an exact snapshot and Propose grant.
    /// - `project`: Project owning the drawing.
    /// - `entity`: Stable semantic identity of the CAD entity.
    /// - `destination_layer`: Stable semantic identity of an existing drawing layer.
    ///
    /// Candidate metadata for shared review, validation, and apply.
    ///
    /// # Returns
    /// The exact-base candidate for the requested layer change.
    ///
    /// # Errors
    /// Returns authorization, stale-base, unsupported-target, or candidate errors.
    pub fn propose_cad_layer_change(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        entity: EntityId,
        destination_layer: EntityId,
    ) -> Result<CadLayerCandidate, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "cad.layer.propose".into(),
                message: "CAD edits require an exact snapshot".into(),
            })?;
        authorize_cad_proposal(context, project, entity, snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "cad.layer.propose".into(),
            }
        })?;
        let candidate = service
            .propose_dxf_layer_change(project, snapshot, entity, destination_layer)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "cad.layer.propose".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(CadLayerCandidate {
            id: candidate.id,
            project: candidate.project,
            base_snapshot: candidate.base_snapshot,
            path: candidate.path,
            validation: candidate.validation,
        })
    }
}

/// Authorizes a CAD proposal for the exact selected entity or a compatibility project grant.
///
/// # Parameters
/// - `context`: Request context containing scoped Propose authority.
/// - `project`: Project owning the CAD entity.
/// - `entity`: Exact semantic entity being changed.
/// - `snapshot`: Exact candidate base snapshot.
///
/// # Returns
/// Unit when entity-scoped or compatibility project-scoped authority applies.
///
/// # Errors
/// Returns a permission error when neither scope authorizes the proposal.
fn authorize_cad_proposal(
    context: &ApplicationContext,
    project: ProjectId,
    entity: EntityId,
    snapshot: scriberig_core::SnapshotId,
) -> Result<(), ApplicationError> {
    let target_scope = CapabilityScope::Entity(entity);
    let project_scope = CapabilityScope::Custom(format!("project:{project}"));
    let operation = Some(OperationKind::DomainOperation);
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
        scope: format!("CAD entity {entity} in project {project}"),
    })
}

/// Checks project-scoped read authority for exact CAD snapshot operations.
///
/// # Parameters
/// - `context`: Trusted request context.
/// - `project`: Project scope to authorize.
/// - `snapshot`: Exact snapshot constraint.
///
/// # Returns
/// Unit when the caller has Inspect permission.
///
/// # Errors
/// Returns a permission error when the scoped grant is absent.
pub(super) fn authorize_cad_inspect(
    context: &ApplicationContext,
    project: ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> Result<(), ApplicationError> {
    let scope = CapabilityScope::Custom(format!("project:{project}"));
    if context.capabilities.permits(
        &CapabilityAction::Inspect,
        &scope,
        Some(snapshot),
        Some(OperationKind::Inspect),
    ) {
        Ok(())
    } else {
        Err(ApplicationError::PermissionDenied {
            action: "Inspect".into(),
            scope: format!("project:{project}"),
        })
    }
}
