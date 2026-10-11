//! Defines exact derivation request identities from profiles and dependencies.

use crate::dependency::DependencySet;
use crate::profile::DerivationProfile;

/// Identifies one derivation request over exact declared dependencies.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DerivationKey {
    /// Exact derivation semantics.
    pub profile: DerivationProfile,
    /// Normalized exact dependencies.
    pub dependencies: DependencySet,
}

// Construct exact derivation keys without implicitly adding global state.
impl DerivationKey {
    /// Creates a key from an exact profile and normalized dependencies.
    ///
    /// # Parameters
    /// - `profile`: Exact derivation definition.
    /// - `dependencies`: Exact declared input set.
    ///
    /// # Returns
    /// A deterministic key. It includes no implicit snapshot dependency.
    #[must_use]
    pub fn new(profile: DerivationProfile, dependencies: DependencySet) -> Self {
        Self {
            profile,
            dependencies,
        }
    }
}
