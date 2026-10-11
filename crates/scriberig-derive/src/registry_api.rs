//! Defines deterministic exact-profile registry operations.

use crate::profile::DerivationProfile;
use crate::registry::{Applicability, DerivationRegistration, RegistrationError};
use std::collections::BTreeMap;

/// Stores registrations indexed by exact profile identity.
///
/// # Public members
/// - `new`: Creates an empty registry.
/// - `register`: Inserts metadata or reports a conflict.
/// - `lookup`: Looks up one exact profile.
/// - `applicable_to`: Lists metadata matching a universal descriptor.
/// - `len`: Returns registration count.
/// - `is_empty`: Reports whether the registry is empty.
///
/// # Private members
/// - `registrations`: Deterministically ordered exact profiles.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivationRegistry {
    registrations: BTreeMap<DerivationProfile, DerivationRegistration>,
}

// Register and inspect exact derivation definitions.
impl DerivationRegistry {
    /// Creates an empty registry.
    ///
    /// # Returns
    /// An empty registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            registrations: BTreeMap::new(),
        }
    }

    /// Registers metadata for an exact profile.
    ///
    /// Identical duplicates are idempotent. Conflicting metadata preserves the
    /// original registration and returns a structured error.
    ///
    /// # Parameters
    /// - `self`: Registry to update.
    /// - `registration`: Metadata to register.
    ///
    /// # Returns
    /// Success when inserted or already present identically.
    ///
    /// # Errors
    /// Returns `ConflictingDuplicate` for conflicting metadata.
    pub fn register(
        &mut self,
        registration: DerivationRegistration,
    ) -> Result<(), RegistrationError> {
        let profile = registration.profile().clone();
        match self.registrations.get(&profile) {
            Some(existing) if existing != &registration => {
                Err(RegistrationError::ConflictingDuplicate { profile })
            }
            Some(_) => Ok(()),
            None => {
                self.registrations.insert(profile, registration);
                Ok(())
            }
        }
    }

    /// Finds metadata by exact profile identity.
    ///
    /// # Parameters
    /// - `self`: Registry to search.
    /// - `profile`: Exact derivation profile.
    ///
    /// # Returns
    /// Matching registration metadata when present.
    #[must_use]
    pub fn lookup(&self, profile: &DerivationProfile) -> Option<&DerivationRegistration> {
        self.registrations.get(profile)
    }

    /// Lists registrations matching one universal applicability descriptor.
    ///
    /// # Parameters
    /// - `self`: Registry to search.
    /// - `applicability`: Entity type or scope kind descriptor.
    ///
    /// # Returns
    /// Matching registrations in exact profile order.
    #[must_use]
    pub fn applicable_to(&self, applicability: &Applicability) -> Vec<&DerivationRegistration> {
        self.registrations
            .values()
            .filter(|value| value.applicability().contains(applicability))
            .collect()
    }

    /// Returns the number of registered profiles.
    ///
    /// # Parameters
    /// - `self`: Registry to inspect.
    ///
    /// # Returns
    /// Number of exact profiles.
    #[must_use]
    pub fn len(&self) -> usize {
        self.registrations.len()
    }

    /// Reports whether the registry has no profiles.
    ///
    /// # Parameters
    /// - `self`: Registry to inspect.
    ///
    /// # Returns
    /// `true` when empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.registrations.is_empty()
    }
}
