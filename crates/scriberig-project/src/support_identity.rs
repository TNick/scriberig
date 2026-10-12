//! Derives stable project-owned source identities and roots.

use scriberig_core::{EntityId, ProjectId, RootName};

use crate::ProjectError;
use crate::errors::invalid_input;

/// Derives a stable source entity identity.
///
/// # Parameters
/// - `project`: Project identity.
/// - `path`: Project-relative path.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Stable source entity identity.
pub(crate) fn source_entity(project: ProjectId, path: &str) -> EntityId {
    let mut bytes = Vec::with_capacity(16 + path.len());
    bytes.extend_from_slice(project.as_bytes());
    bytes.extend_from_slice(path.as_bytes());
    let digest = blake3::hash(&bytes);
    let mut identity = [0; 16];
    identity.copy_from_slice(&digest.as_bytes()[..16]);
    EntityId::from_bytes(identity)
}

/// Creates the named root assigned to one project.
///
/// # Parameters
/// - `project`: Project identity.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Project-specific root name.
///
/// # Errors
/// Returns an invalid root-name error.
pub(crate) fn project_root(project: ProjectId) -> Result<RootName, ProjectError> {
    RootName::new(format!("project_{project}")).map_err(|error| invalid_input(error.to_string()))
}
