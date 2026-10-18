//! Defines validated model and profile library collections.

use std::collections::{BTreeMap, BTreeSet};

use super::model::{ModelDescriptor, ModelId, ModelProfile};

/// Holds the safe model descriptors and curated selectable profiles.
///
/// # Public members
/// - `models`: Provider-neutral model descriptors.
/// - `profiles`: Curated user- or project-selectable profiles.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelLibrary {
    /// Provider-neutral models available to profiles.
    pub models: Vec<ModelDescriptor>,
    /// User- or project-curated model entries.
    pub profiles: Vec<ModelProfile>,
}

impl ModelLibrary {
    /// Creates a validated model library with unique model and profile ids.
    ///
    /// # Parameters
    /// - `models`: Descriptors available to configured profiles.
    /// - `profiles`: Curated entries shown to clients.
    ///
    /// # Returns
    /// A library ordered as supplied by its configuration.
    ///
    /// # Errors
    /// Returns an error for duplicate ids, invalid descriptors, or profiles
    /// that reference missing models or unsupported thinking defaults.
    pub fn new(
        models: Vec<ModelDescriptor>,
        profiles: Vec<ModelProfile>,
    ) -> Result<Self, &'static str> {
        let mut model_map = BTreeMap::new();
        for model in &models {
            model.validate()?;
            if model_map.insert(&model.id, model).is_some() {
                return Err("model ids must be unique");
            }
        }
        let mut profile_ids = BTreeSet::new();
        for profile in &profiles {
            if !profile_ids.insert(&profile.id) {
                return Err("model profile ids must be unique");
            }
            let model = model_map
                .get(&profile.model)
                .ok_or("model profile references a missing model")?;
            profile.validate_for_model(model)?;
        }
        Ok(Self { models, profiles })
    }

    /// Finds a model descriptor by stable identity.
    ///
    /// # Parameters
    /// - `id`: Provider-neutral model identity.
    ///
    /// # Returns
    /// The matching descriptor when it belongs to this library.
    #[must_use]
    pub fn model(&self, id: &ModelId) -> Option<&ModelDescriptor> {
        self.models.iter().find(|model| &model.id == id)
    }

    /// Finds a curated profile by stable identity.
    ///
    /// # Parameters
    /// - `id`: Curated profile identity.
    ///
    /// # Returns
    /// The matching profile when it belongs to this library.
    #[must_use]
    pub fn profile(&self, id: &str) -> Option<&ModelProfile> {
        self.profiles.iter().find(|profile| profile.id == id)
    }
}
