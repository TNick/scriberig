//! Defines compatibility projections for model and profile presentation.

use super::model::ModelDescriptor;

/// Carries model rows for the legacy protocol presentation DTO.
///
/// # Public members
/// - `models`: Projected model rows; upstream owns their metadata.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelLibrary {
    /// Model metadata projected from upstream for legacy presentation.
    pub models: Vec<ModelDescriptor>,
}
