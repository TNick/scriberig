//! Defines provider import provenance separately from CAD semantics.

use super::{CadProviderError, CadProviderErrorKind};
use crate::Drawing;
use scriberig_core::EntityId;
use serde::{Deserialize, Serialize};

/// Carries provider-owned import identifiers beside, not inside, CAD semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadProviderImport {
    /// Provider identity and version that produced this import.
    provider: CadProviderIdentity,
    /// Format-independent semantic drawing.
    drawing: Drawing,
    /// Provider-native identifiers indexed by stable semantic entity identity.
    entity_identifiers: Vec<CadProviderEntityIdentifier>,
    /// Count of source objects intentionally omitted from shared CAD semantics.
    unmapped_object_count: usize,
}

impl CadProviderImport {
    /// Validates and packages provider provenance separately from shared semantics.
    ///
    /// # Parameters
    /// - `provider`: Provider identity and version for this import.
    /// - `drawing`: Semantic drawing without native provider identifiers.
    /// - `entity_identifiers`: Provider-native IDs keyed by semantic entity identity.
    ///
    /// # Returns
    /// A provenance envelope when IDs are unique, refer to imported entities, and are
    /// absent from semantic entity fields.
    ///
    /// # Errors
    /// Returns `InvalidImportProvenance` when a native ID leaks into semantics, an ID
    /// references a missing entity, or IDs are duplicated.
    pub fn new(
        provider: CadProviderIdentity,
        drawing: Drawing,
        entity_identifiers: Vec<CadProviderEntityIdentifier>,
    ) -> Result<Self, CadProviderError> {
        let semantic_ids = drawing
            .entities
            .iter()
            .map(|entity| entity.id)
            .collect::<std::collections::HashSet<_>>();
        let mut external_ids = std::collections::HashSet::new();
        if entity_identifiers.iter().any(|identifier| {
            !semantic_ids.contains(&identifier.entity) || !external_ids.insert(identifier.entity)
        }) {
            return Err(CadProviderError {
                kind: CadProviderErrorKind::InvalidImportProvenance,
            });
        }
        Ok(Self {
            provider,
            drawing,
            entity_identifiers,
            unmapped_object_count: 0,
        })
    }

    /// Marks provider source objects that could not map to the shared CAD schema.
    ///
    /// The exact source artifact remains preserved by project onboarding; a
    /// nonzero count makes the imported drawing explicitly incomplete so callers
    /// cannot mistake the supported semantic subset for a complete import.
    ///
    /// # Parameters
    /// - `count`: Number of source objects omitted from shared semantics.
    ///
    /// # Returns
    /// This import envelope with incomplete-source evidence attached.
    #[must_use]
    pub fn with_unmapped_object_count(mut self, count: usize) -> Self {
        self.unmapped_object_count = count;
        self
    }

    /// Returns the provider identity recorded for this import.
    ///
    /// # Returns
    /// Provider identifier and version, without deployment secrets.
    #[must_use]
    pub fn provider(&self) -> &CadProviderIdentity {
        &self.provider
    }

    /// Returns shared CAD semantics without provider-native handles.
    ///
    /// # Returns
    /// The validated format-independent drawing.
    #[must_use]
    pub fn drawing(&self) -> &Drawing {
        &self.drawing
    }

    /// Returns provider-native entity identifiers as external provenance.
    ///
    /// # Returns
    /// The identifiers keyed by semantic entity IDs.
    #[must_use]
    pub fn entity_identifiers(&self) -> &[CadProviderEntityIdentifier] {
        &self.entity_identifiers
    }

    /// Returns the number of source objects omitted from shared CAD semantics.
    ///
    /// # Returns
    /// Zero for a provider-declared complete import, otherwise the omitted count.
    #[must_use]
    pub fn unmapped_object_count(&self) -> usize {
        self.unmapped_object_count
    }

    /// Returns provider-owned identity evidence for one semantic entity.
    ///
    /// # Parameters
    /// - `entity`: Stable semantic entity identity.
    ///
    /// # Returns
    /// The detached provider identifier when one was recorded for this entity.
    #[must_use]
    pub fn entity_identifier(&self, entity: EntityId) -> Option<&CadProviderEntityIdentifier> {
        self.entity_identifiers
            .iter()
            .find(|identifier| identifier.entity == entity)
    }
}

/// Identifies a provider build without exposing deployment secrets.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CadProviderIdentity {
    /// Stable provider identifier.
    pub id: String,
    /// Provider version used for this import.
    pub version: String,
}

/// Keeps one provider-native object identifier in import provenance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CadProviderEntityIdentifier {
    /// Semantic entity this external object identifies in this import.
    pub entity: EntityId,
    /// Provider-native identifier, opaque to shared CAD semantics.
    pub value: String,
}
