//! Defines runtime capability metadata without implementing authorization policy.

use std::fmt::{self, Display, Formatter};

/// Identifies a capability exposed by the current application configuration.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CapabilityId(String);

impl CapabilityId {
    /// Creates a capability identifier from a nonempty name.
    ///
    /// # Parameters
    /// - `name`: Stable capability name.
    ///
    /// # Returns
    /// The capability identifier.
    ///
    /// # Errors
    /// Returns an error when `name` is empty or whitespace-only.
    pub fn new(name: impl Into<String>) -> Result<Self, &'static str> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err("capability name must not be empty");
        }
        Ok(Self(name))
    }

    /// Borrows the stable capability name.
    ///
    /// # Parameters
    /// - `self`: Capability identifier to inspect.
    ///
    /// # Returns
    /// The capability name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for CapabilityId {
    /// Formats the stable capability name.
    ///
    /// # Parameters
    /// - `self`: Capability identifier to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns a formatting error when the destination rejects the write.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Describes one capability available from an application configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeCapability {
    /// Stable capability identifier.
    pub id: CapabilityId,
    /// Short human-readable capability description.
    pub description: String,
    /// Whether the capability is available in this configuration.
    pub available: bool,
    /// Optional domain or scope metadata.
    pub scope: Option<String>,
}

impl RuntimeCapability {
    /// Creates a runtime capability descriptor.
    ///
    /// # Parameters
    /// - `id`: Stable capability identifier.
    /// - `description`: Short capability description.
    /// - `available`: Availability in the current configuration.
    ///
    /// # Returns
    /// A capability descriptor without an optional scope.
    #[must_use]
    pub fn new(id: CapabilityId, description: impl Into<String>, available: bool) -> Self {
        Self {
            id,
            description: description.into(),
            available,
            scope: None,
        }
    }

    /// Adds optional domain or scope metadata.
    ///
    /// # Parameters
    /// - `self`: Capability descriptor to update.
    /// - `scope`: Scope metadata shown to callers.
    ///
    /// # Returns
    /// The updated descriptor.
    #[must_use]
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }
}
