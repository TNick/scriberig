//! Defines exact, versioned derivation profile identities and scheduling hints.

use std::fmt::{self, Display, Formatter};

/// Suggests when a derivation should be computed.
///
/// This is a scheduling hint only and does not affect result validity.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DerivationSchedule {
    /// Compute only when a consumer requests the result.
    #[default]
    Lazy,
    /// Prefer maintaining the result as inputs change.
    Eager,
}

/// Identifies one exact version of a domain-neutral derivation definition.
///
/// # Public members
/// - `new`: Creates a profile from its stable kind and version identifiers.
/// - `with_producer`: Adds an implementation identity to the profile.
/// - `with_configuration`: Adds an exact semantic configuration identity.
/// - `with_schedule`: Sets the profile scheduling hint.
/// - `kind`: Returns the extensible derivation kind.
/// - `version`: Returns the exact derivation version.
/// - `producer`: Returns the optional producer identity.
/// - `configuration`: Returns the optional configuration identity.
/// - `schedule`: Returns the eager/lazy scheduling hint.
///
/// # Private members
/// - `kind`: Stores the domain-neutral profile kind.
/// - `version`: Stores the exact profile version.
/// - `producer`: Stores an optional implementation identity.
/// - `configuration`: Stores an optional exact configuration identity.
/// - `schedule`: Stores a non-validity scheduling hint.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DerivationProfile {
    kind: String,
    version: String,
    producer: Option<String>,
    configuration: Option<String>,
    schedule: DerivationSchedule,
}

// Construct and inspect exact derivation profile values.
impl DerivationProfile {
    /// Creates a profile with a stable kind and exact version.
    ///
    /// # Parameters
    /// - `kind`: Extensible derivation kind identifier.
    /// - `version`: Exact semantic version or revision label.
    ///
    /// # Returns
    /// The exact profile with lazy scheduling and no optional identities.
    #[must_use]
    pub fn new(kind: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            version: version.into(),
            producer: None,
            configuration: None,
            schedule: DerivationSchedule::default(),
        }
    }

    /// Adds an exact producer or implementation identity.
    ///
    /// # Parameters
    /// - `self`: Profile being configured.
    /// - `producer`: Stable implementation identity.
    ///
    /// # Returns
    /// The profile with its producer identity set.
    #[must_use]
    pub fn with_producer(mut self, producer: impl Into<String>) -> Self {
        self.producer = Some(producer.into());
        self
    }

    /// Adds an exact semantic configuration identity.
    ///
    /// # Parameters
    /// - `self`: Profile being configured.
    /// - `configuration`: Stable configuration identity.
    ///
    /// # Returns
    /// The profile with its configuration identity set.
    #[must_use]
    pub fn with_configuration(mut self, configuration: impl Into<String>) -> Self {
        self.configuration = Some(configuration.into());
        self
    }

    /// Sets the profile's eager or lazy scheduling hint.
    ///
    /// # Parameters
    /// - `self`: Profile being configured.
    /// - `schedule`: Scheduling preference.
    ///
    /// # Returns
    /// The profile with the requested scheduling hint.
    #[must_use]
    pub const fn with_schedule(mut self, schedule: DerivationSchedule) -> Self {
        self.schedule = schedule;
        self
    }

    /// Returns the extensible derivation kind identifier.
    ///
    /// # Parameters
    /// - `self`: Profile to inspect.
    ///
    /// # Returns
    /// The derivation kind.
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns the exact derivation version.
    ///
    /// # Parameters
    /// - `self`: Profile to inspect.
    ///
    /// # Returns
    /// The derivation version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the optional producer identity.
    ///
    /// # Parameters
    /// - `self`: Profile to inspect.
    ///
    /// # Returns
    /// The producer identity, when specified.
    #[must_use]
    pub fn producer(&self) -> Option<&str> {
        self.producer.as_deref()
    }

    /// Returns the optional semantic configuration identity.
    ///
    /// # Parameters
    /// - `self`: Profile to inspect.
    ///
    /// # Returns
    /// The configuration identity, when specified.
    #[must_use]
    pub fn configuration(&self) -> Option<&str> {
        self.configuration.as_deref()
    }

    /// Returns the profile's scheduling hint.
    ///
    /// # Parameters
    /// - `self`: Profile to inspect.
    ///
    /// # Returns
    /// The eager/lazy scheduling preference.
    #[must_use]
    pub const fn schedule(&self) -> DerivationSchedule {
        self.schedule
    }
}

// Format profiles for diagnostics without making display text their identity.
impl Display for DerivationProfile {
    /// Formats the profile kind and exact version.
    ///
    /// # Parameters
    /// - `self`: Profile to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns an error if writing to the formatter fails.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}@{}", self.kind, self.version)
    }
}
