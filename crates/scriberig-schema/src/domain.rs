//! Defines stable descriptors for installed semantic domains.

use crate::CapabilityDescriptor;
use scriberig_core::ObjectType;

/// Identifies an installed domain independently of its Rust crate name.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DomainId(String);

impl DomainId {
    /// Validates and constructs a domain identifier.
    ///
    /// # Parameters
    /// - `value`: Stable nonempty domain identifier.
    ///
    /// # Returns
    /// The domain identifier.
    ///
    /// # Errors
    /// Returns an error when the identifier is empty or whitespace-only.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("domain identifiers must not be empty");
        }
        Ok(Self(value))
    }

    /// Borrows the stable domain identifier.
    ///
    /// # Parameters
    /// - `self`: Domain identifier to inspect.
    ///
    /// # Returns
    /// The identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Describes one installed semantic domain and its advertised functionality.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainDescriptor {
    /// Stable domain identifier.
    pub id: DomainId,
    /// Human-readable domain name.
    pub name: String,
    /// Version of the domain schema contract.
    pub schema_version: u32,
    /// Entity types supplied by this domain.
    pub entity_types: Vec<ObjectType>,
    /// Relation identifiers supplied by this domain.
    pub relation_types: Vec<String>,
    /// Capabilities supplied by this domain.
    pub capabilities: Vec<CapabilityDescriptor>,
}
