//! Defines actor identity independently from authentication and permission.

use std::fmt::{self, Display, Formatter};

/// Identifies an actor for attribution without embedding authority.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PrincipalId(String);

impl PrincipalId {
    /// Validates and constructs a principal identifier.
    ///
    /// # Parameters
    /// - `value`: Stable nonempty actor identifier.
    ///
    /// # Returns
    /// The principal identity.
    ///
    /// # Errors
    /// Returns an error when `value` is empty or whitespace-only.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("principal identifiers must not be empty");
        }
        Ok(Self(value))
    }

    /// Borrows the stable principal identifier.
    ///
    /// # Parameters
    /// - `self`: Principal identity to inspect.
    ///
    /// # Returns
    /// The identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for PrincipalId {
    /// Formats the principal identifier.
    ///
    /// # Parameters
    /// - `self`: Principal identity to format.
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

/// Classifies the actor that owns a principal identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrincipalKind {
    /// A human actor.
    Human,
    /// An autonomous or assisted agent.
    Agent,
    /// A trusted system component.
    System,
    /// A long-lived service.
    Service,
    /// An external connector.
    Connector,
    /// An import process.
    Importer,
    /// A validation process.
    Validator,
    /// An API client.
    ApiClient,
    /// An extensible actor category.
    Custom(String),
}
