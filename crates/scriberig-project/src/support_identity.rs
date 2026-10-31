//! Derives stable project-owned source identities and roots.

use scriberig_core::{ProjectId, RootName};

use crate::ProjectError;
use crate::errors::invalid_input;

/// Creates the named root assigned to one project.
///
/// # Parameters
/// - `project`: Project identity.
/// - `crate`: Internal visibility marker.
///
/// Project-specific root name.
///
/// Returns an invalid root-name error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn project_root(project: ProjectId) -> Result<RootName, ProjectError> {
    RootName::new(format!("project_{project}")).map_err(|error| invalid_input(error.to_string()))
}
