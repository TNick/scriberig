//! Defines runtime metadata for a filesystem-backed projection.

use crate::{FileDigest, ProjectionLocation, ProjectionOwnership};
use sr_projection::{ProjectionRef, ProjectionStatus, RendererVersion, SemanticStateRef, StyleRef};

/// Runtime synchronization state kept outside immutable projection configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilesystemProjectionState {
    /// Projection instance.
    pub projection: ProjectionRef,
    /// Physical location.
    pub location: ProjectionLocation,
    /// Ownership policy.
    pub ownership: ProjectionOwnership,
    /// Last exact synchronized semantic state.
    pub base: SemanticStateRef,
    /// Last rendered bytes digest.
    pub rendered_digest: FileDigest,
    /// Last observed bytes digest, when observed.
    pub observed_digest: Option<FileDigest>,
    /// Current generic lifecycle status.
    pub status: ProjectionStatus,
    /// Exact style input used for interpretation.
    pub style: StyleRef,
    /// Exact renderer version used for interpretation.
    pub renderer: RendererVersion,
}
