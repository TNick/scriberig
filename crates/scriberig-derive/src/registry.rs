//! Registers generic derivation metadata for domain and application consumers.

use crate::freshness::FreshnessPolicy;
use crate::profile::{DerivationProfile, DerivationSchedule};
use crate::retention::RetentionPolicy;
use std::fmt::{self, Display, Formatter};

pub use crate::registry_api::DerivationRegistry;

#[cfg(test)]
#[path = "registry_test.rs"]
mod tests;

/// Identifies accepted universal entity types or scope kinds.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Applicability {
    /// Applies to entities with this opaque universal type ID.
    EntityType(String),
    /// Applies to scopes with this opaque universal kind ID.
    ScopeKind(String),
}

/// Describes how a derivation obtains its exact dependency set.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DependencyDiscovery {
    /// Caller declares exact inputs.
    DeclaredExactInputs,
    /// Complete declared snapshot is an input.
    WholeSnapshot,
    /// Producer supplies dependencies through the generic dependency model.
    ProducerSupplied,
    /// Adapter-defined, inspectable strategy identifier.
    Extension(String),
}

/// Identifies a stable implementation or producer.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProducerIdentity(String);

// Construct and inspect producer identities.
impl ProducerIdentity {
    /// Creates an identity from a stable opaque string.
    ///
    /// - `identity`: Stable implementation identifier.
    ///
    /// The producer identity.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(identity: impl Into<String>) -> Self {
        Self(identity.into())
    }

    /// Returns the stable identity string.
    ///
    /// - `self`: Identity to inspect.
    ///
    /// The underlying identifier.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Provides a producer-defined relative cost estimate.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CostHint {
    /// Relative units chosen by the registering adapter.
    pub units: u64,
}

/// Describes one derivation and its generic infrastructure defaults.
///
/// # Public members
/// - `new`: Creates a registration.
/// - `with_schedule`: Overrides the profile schedule.
/// - `with_freshness`: Sets the freshness default.
/// - `with_retention`: Sets the retention default.
/// - `with_cost`: Sets the optional cost estimate.
/// - `profile`: Returns the exact profile.
/// - `applicability`: Returns compatibility descriptors.
/// - `dependency_discovery`: Returns the dependency strategy.
/// - `schedule`: Returns the effective schedule.
/// - `freshness`: Returns the freshness policy.
/// - `retention`: Returns the retention policy.
/// - `producer`: Returns the producer identity.
/// - `cost`: Returns the optional cost hint.
///
/// # Private members
/// - `profile`: Exact profile identity.
/// - `applicability`: Universal compatibility descriptors.
/// - `dependency_discovery`: Inspectable dependency contract.
/// - `schedule`: Optional scheduling override.
/// - `freshness`: Freshness default.
/// - `retention`: Retention default.
/// - `producer`: Implementation identity.
/// - `cost`: Optional relative cost estimate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationRegistration {
    profile: DerivationProfile,
    applicability: Vec<Applicability>,
    dependency_discovery: DependencyDiscovery,
    schedule: Option<DerivationSchedule>,
    freshness: FreshnessPolicy,
    retention: RetentionPolicy,
    producer: ProducerIdentity,
    cost: Option<CostHint>,
}

// Construct and inspect registration metadata.
impl DerivationRegistration {
    /// Creates a registration with generic freshness and retention defaults.
    ///
    /// - `profile`: Exact profile definition.
    /// - `applicability`: Universal subject/type descriptors.
    /// - `dependency_discovery`: Explicit dependency strategy.
    /// - `producer`: Implementation identity.
    ///
    /// A registration defaulting to profile schedule, immutable freshness,
    /// and disposable retention.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(
        profile: DerivationProfile,
        applicability: Vec<Applicability>,
        dependency_discovery: DependencyDiscovery,
        producer: ProducerIdentity,
    ) -> Self {
        Self {
            profile,
            applicability,
            dependency_discovery,
            schedule: None,
            freshness: FreshnessPolicy::Immutable,
            retention: RetentionPolicy::Disposable,
            producer,
            cost: None,
        }
    }

    /// Overrides the profile scheduling hint.
    ///
    /// - `self`: Registration being configured.
    /// - `schedule`: Eager/lazy preference.
    ///
    /// The configured registration.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_schedule(mut self, schedule: DerivationSchedule) -> Self {
        self.schedule = Some(schedule);
        self
    }

    /// Sets the registration freshness default.
    ///
    /// - `self`: Registration being configured.
    /// - `freshness`: Freshness policy.
    ///
    /// The configured registration.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_freshness(mut self, freshness: FreshnessPolicy) -> Self {
        self.freshness = freshness;
        self
    }

    /// Sets the registration retention default.
    ///
    /// - `self`: Registration being configured.
    /// - `retention`: Retention policy.
    ///
    /// The configured registration.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_retention(mut self, retention: RetentionPolicy) -> Self {
        self.retention = retention;
        self
    }

    /// Sets the optional relative cost estimate.
    ///
    /// - `self`: Registration being configured.
    /// - `cost`: Cost estimate.
    ///
    /// The configured registration.
    #[must_use]
    pub const fn with_cost(mut self, cost: CostHint) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Returns the exact derivation profile.
    ///
    /// - `self`: Registration to inspect.
    ///
    /// The exact profile.
    #[must_use]
    pub const fn profile(&self) -> &DerivationProfile {
        &self.profile
    }

    /// Returns universal applicability descriptors.
    ///
    /// - `self`: Registration to inspect.
    ///
    /// Accepted type and scope descriptors.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn applicability(&self) -> &[Applicability] {
        &self.applicability
    }

    /// Returns dependency discovery metadata.
    ///
    /// - `self`: Registration to inspect.
    ///
    /// The explicit dependency strategy.
    #[must_use]
    pub const fn dependency_discovery(&self) -> &DependencyDiscovery {
        &self.dependency_discovery
    }

    /// Returns the effective scheduling hint.
    ///
    /// - `self`: Registration to inspect.
    ///
    /// The override or profile scheduling hint.
    #[must_use]
    pub const fn schedule(&self) -> DerivationSchedule {
        match self.schedule {
            Some(value) => value,
            None => self.profile.schedule(),
        }
    }

    /// Returns the freshness default.
    ///
    /// - `self`: Registration to inspect.
    ///
    /// The freshness policy.
    #[must_use]
    pub const fn freshness(&self) -> &FreshnessPolicy {
        &self.freshness
    }

    /// Returns the retention default.
    ///
    /// - `self`: Registration to inspect.
    ///
    /// The retention policy.
    #[must_use]
    pub const fn retention(&self) -> RetentionPolicy {
        self.retention
    }

    /// Returns the producer identity.
    ///
    /// - `self`: Registration to inspect.
    ///
    /// The implementation identity.
    #[must_use]
    pub const fn producer(&self) -> &ProducerIdentity {
        &self.producer
    }

    /// Returns the optional cost estimate.
    ///
    /// - `self`: Registration to inspect.
    ///
    /// The cost hint when present.
    #[must_use]
    pub const fn cost(&self) -> Option<CostHint> {
        self.cost
    }
}

/// Reports a conflict while registering an exact profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistrationError {
    /// Existing metadata differs for this exact profile.
    ConflictingDuplicate {
        /// Exact profile with conflicting metadata.
        profile: DerivationProfile,
    },
}

// Format structured registration failures.
impl Display for RegistrationError {
    /// Formats the registration failure.
    ///
    /// - `self`: Error to format.
    /// - `formatter`: Destination formatter.
    ///
    /// A formatting result.
    ///
    /// Returns an error if writing to the formatter fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingDuplicate { profile } => {
                write!(formatter, "conflicting registration for profile {profile}")
            }
        }
    }
}

// Implement the standard error contract.
impl std::error::Error for RegistrationError {}
