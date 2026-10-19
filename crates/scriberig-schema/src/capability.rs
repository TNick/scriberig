//! Defines stable, transport-neutral domain capability metadata.

use std::fmt::{self, Display, Formatter};

/// Identifies a semantic capability independently of its implementation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CapabilityId(String);

impl CapabilityId {
    /// Validates and constructs a capability identifier.
    ///
    /// - `value`: Stable capability name using letters, digits, `.`, `_`, or `-`.
    ///
    /// The validated capability identifier.
    ///
    /// Returns an error when the value is empty or contains an unsupported character.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.is_empty()
            || !value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
        {
            return Err("capability identifiers must be nonempty and transport-safe");
        }
        Ok(Self(value))
    }

    /// Borrows the stable capability identifier.
    ///
    /// - `self`: Capability identifier to inspect.
    ///
    /// The identifier text.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for CapabilityId {
    /// Formats the capability identifier.
    ///
    /// - `self`: Capability identifier to format.
    /// - `formatter`: Destination formatter.
    ///
    /// A formatting result.
    ///
    /// Returns a formatting error when the destination rejects the write.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Groups capabilities advertised for an entity type.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CapabilityCategory {
    /// A typed relation can be followed.
    Relation,
    /// A named child or collection view is available.
    Collection,
    /// A semantic description or summary can be produced.
    Description,
    /// A projection can render or synchronize the entity.
    Projection,
    /// A semantic command is available.
    Command,
    /// An analysis can inspect or derive information.
    Analysis,
    /// A validator can produce evidence.
    Validation,
}

/// Describes one capability available for a semantic entity type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityDescriptor {
    /// Stable semantic capability identifier.
    pub id: CapabilityId,
    /// Entity type for which the capability is available.
    pub entity_type: scriberig_core::ObjectType,
    /// Capability category exposed to generic clients.
    pub category: CapabilityCategory,
    /// Short human-readable description.
    pub description: String,
    /// Provider metadata when the capability comes from an adapter.
    pub provider: Option<ProviderMetadata>,
}

/// Identifies the implementation that provides a capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderMetadata {
    /// Stable provider or adapter identifier.
    pub id: String,
    /// Provider release or profile version.
    pub version: String,
    /// Optional semantics-affecting profile identifier.
    pub profile: Option<String>,
    /// Whether the provider is currently configured and usable.
    pub available: bool,
}

impl ProviderMetadata {
    /// Creates provider metadata without a profile.
    ///
    /// - `id`: Stable provider or adapter identifier.
    /// - `version`: Provider version.
    /// - `available`: Whether the provider is usable.
    ///
    /// Provider metadata with no profile binding.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(id: impl Into<String>, version: impl Into<String>, available: bool) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
            profile: None,
            available,
        }
    }

    /// Binds a semantics-affecting provider profile.
    ///
    /// - `self`: Provider metadata to update.
    /// - `profile`: Stable profile identifier.
    ///
    /// Metadata with the profile attached.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }
}
