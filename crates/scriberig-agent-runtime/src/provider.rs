//! Defines ScribeRig-owned host configuration diagnostics.

use std::fmt;

/// Reports invalid host-side provider configuration or scenario setup.
///
/// Provider transport, authentication, request shaping, and response
/// normalization failures are represented by upstream `rpi-ai` APIs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostConfigurationError {
    /// Host-owned configuration is missing or invalid.
    Configuration(String),
    /// A `ScribeRig` semantic scenario lacks required `ScribeRig` state.
    Scenario(String),
}

impl fmt::Display for HostConfigurationError {
    /// Formats a host configuration diagnostic.
    ///
    /// # Parameters
    /// - `self`: Diagnostic to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatting success or the formatter error.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(message) => write!(formatter, "configuration: {message}"),
            Self::Scenario(message) => write!(formatter, "scenario: {message}"),
        }
    }
}

impl std::error::Error for HostConfigurationError {}

impl From<String> for HostConfigurationError {
    /// Converts a scenario message to a host configuration error.
    ///
    /// # Parameters
    /// - `message`: Safe scenario failure description.
    ///
    /// # Returns
    /// A scenario error containing the message.
    fn from(message: String) -> Self {
        Self::Scenario(message)
    }
}
