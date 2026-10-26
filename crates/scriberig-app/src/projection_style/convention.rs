//! Defines the pluggable boundary through which repository conventions feed style.

use sr_projection::{ProjectionKind, StyleValue};

use crate::ApplicationError;

/// Contributes a style input inferred from repository-level conventions.
///
/// Task 151 deliberately stops at the boundary: interpreting `.editorconfig`,
/// rustfmt, Prettier, or Biome files is domain work owned by the projection task
/// that renders those languages, and any importer must produce a canonical
/// domain style input rather than letting a formatter read ambient configuration
/// as hidden render truth. Unsupported options must be surfaced, not silently
/// approximated.
pub trait RepositoryConventionSource: Send + Sync {
    /// Returns the style input this source contributes for one project and kind.
    ///
    /// - `self`: Convention source being consulted.
    /// - `project_id`: Stable project identity whose conventions apply.
    /// - `kind`: Projection kind being rendered.
    ///
    /// # Returns
    /// The canonical style input when this source declares one for the pair.
    ///
    /// # Errors
    /// Returns an application error when the convention cannot be read or an
    /// option it would need is unsupported.
    fn convention_style(
        &self,
        project_id: &str,
        kind: &ProjectionKind,
    ) -> Result<Option<StyleValue>, ApplicationError>;
}

/// A fixed synthetic convention source used by tests and deterministic hosts.
///
/// It exists so precedence behavior can be proven without inventing a partial
/// configuration-file importer inside the shared layer.
///
/// # Public members
/// - `new`: Builds a source for one project, kind, and value.
///
/// # Private members
/// - `project_id`: Project whose conventions this source represents.
/// - `projection_kind`: Kind the value applies to.
/// - `value`: Canonical style input contributed.
pub struct StaticRepositoryConvention {
    /// Project whose conventions this source represents.
    project_id: String,
    /// Kind the value applies to.
    projection_kind: ProjectionKind,
    /// Canonical style input contributed.
    value: StyleValue,
}

impl StaticRepositoryConvention {
    /// Builds one fixed convention contribution.
    ///
    /// - `project_id`: Project whose conventions this source represents.
    /// - `projection_kind`: Kind the value applies to.
    /// - `value`: Canonical style input contributed.
    ///
    /// # Returns
    /// The convention source.
    #[must_use]
    pub fn new(
        project_id: impl Into<String>,
        projection_kind: ProjectionKind,
        value: StyleValue,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            projection_kind,
            value,
        }
    }
}

impl RepositoryConventionSource for StaticRepositoryConvention {
    /// Returns the fixed contribution when the project and kind match.
    ///
    /// - `self`: Convention source being consulted.
    /// - `project_id`: Stable project identity whose conventions apply.
    /// - `kind`: Projection kind being rendered.
    ///
    /// # Returns
    /// The contributed style input for the matching pair.
    ///
    /// # Errors
    /// Never fails for this source.
    fn convention_style(
        &self,
        project_id: &str,
        kind: &ProjectionKind,
    ) -> Result<Option<StyleValue>, ApplicationError> {
        if project_id != self.project_id || kind != &self.projection_kind {
            return Ok(None);
        }
        Ok(Some(self.value.clone()))
    }
}
