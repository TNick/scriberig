//! Materializes managed projection files under one purpose-aware shared style.
//!
//! Materialization is the product path that turns canonical state into files. The
//! style that those files carry must come from the shared purpose-aware
//! resolution, and only the application layer holds the installed style adapter
//! and configuration store, so this module owns the operation that pairs a
//! `SharedMaterialization` resolution with the filesystem write owned by
//! `sr-projection-fs`. No product surface may resolve a materialization style for
//! itself, and the filesystem layer never consults style configuration: the exact
//! resolved value is handed down.
//!
//! The dependency direction stays intact: `sr-projection-fs` is a foundation
//! projection crate above `sr-projection`, so the application layer may call it
//! while nothing below the application layer learns about the application.

use sr_projection::{
    ProjectionAdapter, ProjectionPurpose, ProjectionRef, ResolvedStyle, StyleSource,
};
use sr_projection_fs::{
    FileDigest, FileSystem, IoError, ProjectionLocation, RenderError, SyncCoordinator, digest_bytes,
};

use crate::projection_style::{ProjectionStyleBackend, StyleResolutionRequest};
use crate::{ApplicationContext, ApplicationError, ApplicationService};

/// Asks the application to materialize one managed projection file.
///
/// The projection kind and file location are not supplied here: they come from the
/// projection's recorded synchronization state, so a materialization can never
/// resolve a style for a kind that differs from the file it writes.
///
/// # Public members
/// - `projection`: Managed projection whose synchronization state names the file.
/// - `project_id`: Stable project identity whose style policy applies.
/// - `workspace_id`: Stable workspace identity whose materialization policy applies.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionMaterializationRequest {
    /// Managed projection whose synchronization state names the file.
    pub projection: ProjectionRef,
    /// Stable project identity whose style policy applies.
    pub project_id: String,
    /// Stable workspace identity whose materialization policy applies.
    pub workspace_id: String,
}

/// Reports one materialization the application performed.
///
/// # Public members
/// - `projection`: Projection whose file was written.
/// - `location`: Exact managed file that was written.
/// - `resolved`: Exact style the written bytes carry.
/// - `applied`: Ordered sources that produced that style.
/// - `rendered`: Exact-byte digest recorded for the written file.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializedProjection {
    /// Projection whose file was written.
    pub projection: ProjectionRef,
    /// Exact managed file that was written.
    pub location: ProjectionLocation,
    /// Exact style the written bytes carry.
    pub resolved: ResolvedStyle,
    /// Ordered sources that produced that style.
    pub applied: Vec<StyleSource>,
    /// Exact-byte digest recorded for the written file.
    pub rendered: FileDigest,
}

impl<B> ApplicationService<B> {
    /// Materializes one managed filesystem projection under the shared style.
    ///
    /// Resolution runs for the projection's recorded kind with the
    /// `SharedMaterialization` purpose, so the resolver excludes personal
    /// preference and view state by construction: materialized files cannot pick
    /// up whoever happens to be logged in. The exact resolved value is then handed
    /// to `sr-projection-fs`, which writes the file and records that style as the
    /// render baseline for later observation and reconciliation.
    ///
    /// - `self`: Service holding the installed style adapter and style store.
    /// - `context`: Caller context; its personal principal is deliberately unused.
    /// - `request`: Projection plus the project and workspace identities to resolve.
    /// - `adapter`: Domain adapter that renders the projection's recorded kind.
    /// - `coordinator`: Filesystem boundary owning that projection's state.
    ///
    /// # Returns
    /// The written location, the exact applied style with its ordered sources, and
    /// the digest recorded for the written bytes.
    ///
    /// # Errors
    /// Returns not-found when the projection has no recorded synchronization state,
    /// the resolution errors of the shared style operation (unsupported kind,
    /// schema violation, or store failure), an invalid-request error when the
    /// filesystem boundary refuses the write on ownership grounds, and an internal
    /// error when rendering or the filesystem write fails.
    pub fn materialize_projection<F, A>(
        &self,
        context: &ApplicationContext,
        request: &ProjectionMaterializationRequest,
        adapter: &A,
        coordinator: &mut SyncCoordinator<F>,
    ) -> Result<MaterializedProjection, ApplicationError>
    where
        F: FileSystem,
        A: ProjectionAdapter,
    {
        let state = coordinator
            .states()
            .find_projection(request.projection)
            .cloned()
            .ok_or_else(|| not_found(request.projection))?;
        let effective = self.projection_style_effective(
            context,
            &StyleResolutionRequest::new(
                state.baseline.rendered.kind,
                request.project_id.clone(),
                request.workspace_id.clone(),
                ProjectionPurpose::SharedMaterialization,
            ),
        )?;
        coordinator
            .render_managed_projection_with_style(request.projection, adapter, &effective.resolved)
            .map_err(|error| materialization_error(request.projection, error))?;
        let written = coordinator
            .states()
            .find_projection(request.projection)
            .ok_or_else(|| not_found(request.projection))?;
        Ok(MaterializedProjection {
            projection: request.projection,
            location: written.location.clone(),
            resolved: effective.resolved,
            applied: effective.applied,
            rendered: digest_bytes(&written.baseline.rendered.bytes),
        })
    }
}

/// Names a missing synchronization state for one projection.
///
/// - `projection`: Projection whose filesystem state was absent.
///
/// # Returns
/// The not-found application error.
fn not_found(projection: ProjectionRef) -> ApplicationError {
    ApplicationError::NotFound {
        subject: format!("projection {}", projection.0),
    }
}

/// Maps one filesystem materialization failure into an application error.
///
/// - `projection`: Projection the failed write targeted.
/// - `error`: Failure reported by the filesystem projection layer.
///
/// # Returns
/// The structured application error matching the failure category.
fn materialization_error(projection: ProjectionRef, error: RenderError) -> ApplicationError {
    match error {
        RenderError::Io(IoError::OwnershipDenied) => ApplicationError::InvalidRequest {
            operation: "projection materialize".to_owned(),
            message: format!(
                "projection {} is not a writable managed projection",
                projection.0
            ),
        },
        RenderError::Io(error) => ApplicationError::Internal {
            message: format!("materializing projection {} failed: {error}", projection.0),
        },
        RenderError::Diagnostic(diagnostic) => crate::projection_style::resolver::style_error(
            "projection materialize render",
            diagnostic,
        ),
    }
}
