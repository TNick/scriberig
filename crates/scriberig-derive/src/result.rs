//! Defines immutable derived result records and structured attempt status.

use crate::event::DerivedResultHandle;
use crate::freshness::FreshnessObservation;
use crate::key::DerivationKey;
use crate::retention::{RetentionPolicy, RetentionStrength};

/// Classifies whether a derivation attempt succeeded or can be reused.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DerivedResultStatus {
    /// A usable result was produced.
    Succeeded,
    /// An immutable-input failure may be inspected or reused deterministically.
    FailedDeterministic,
    /// A retryable failure should not suppress a later attempt.
    FailedTransient,
    /// Work did not complete and should not satisfy a request.
    Cancelled,
}

/// Identifies structured failure information without parsing display prose.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FailureReason {
    /// Input did not satisfy derivation requirements.
    InvalidInput,
    /// An external dependency was unavailable temporarily.
    TemporaryUnavailable,
    /// Work was cancelled before producing a result.
    Cancelled,
    /// Adapter-defined machine-readable failure code.
    Extension(String),
}

/// Stores one immutable derivation outcome and its exact inputs.
///
/// # Public members
/// - `new`: Creates a result record.
/// - `with_freshness`: Attaches an external observation.
/// - `with_failure`: Attaches a structured failure reason.
/// - `with_retention`: Sets its retention category and strength.
/// - `key`: Returns the exact derivation key.
/// - `handle`: Returns the immutable result handle.
/// - `status`: Returns structured attempt status.
/// - `freshness`: Returns optional observation metadata.
/// - `failure`: Returns optional structured failure.
/// - `retention`: Returns retention metadata.
///
/// # Private members
/// - `key`: Exact profile and dependency identity.
/// - `handle`: Immutable artifact identity.
/// - `status`: Structured success/failure category.
/// - `freshness`: Optional source observation data.
/// - `failure`: Optional machine-readable failure reason.
/// - `retention`: Storage policy independent of validity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedResultRecord {
    key: DerivationKey,
    handle: DerivedResultHandle,
    status: DerivedResultStatus,
    freshness: Option<FreshnessObservation>,
    failure: Option<FailureReason>,
    retention: (RetentionPolicy, RetentionStrength),
}

// Construct and inspect immutable derivation outcomes.
impl DerivedResultRecord {
    /// Creates an immutable result record.
    ///
    /// # Parameters
    /// - `key`: Exact derivation request identity.
    /// - `handle`: Immutable artifact identity.
    /// - `status`: Structured attempt outcome.
    ///
    /// # Returns
    /// A record with disposable retention and weak input references.
    #[must_use]
    pub const fn new(
        key: DerivationKey,
        handle: DerivedResultHandle,
        status: DerivedResultStatus,
    ) -> Self {
        Self {
            key,
            handle,
            status,
            freshness: None,
            failure: None,
            retention: (
                RetentionPolicy::Disposable,
                RetentionStrength::WeakDependency,
            ),
        }
    }

    /// Associates an exact historical source observation with this record.
    ///
    /// # Parameters
    /// - `self`: Record being configured.
    /// - `observation`: Immutable observation and freshness deadline.
    ///
    /// # Returns
    /// The record with observation metadata attached.
    #[must_use]
    pub fn with_freshness(mut self, observation: FreshnessObservation) -> Self {
        self.freshness = Some(observation);
        self
    }

    /// Attaches a machine-readable failure reason.
    ///
    /// # Parameters
    /// - `self`: Record being configured.
    /// - `reason`: Structured failure category.
    ///
    /// # Returns
    /// The record with failure metadata attached.
    #[must_use]
    pub fn with_failure(mut self, reason: FailureReason) -> Self {
        self.failure = Some(reason);
        self
    }

    /// Sets retention category and canonical input reference strength.
    ///
    /// # Parameters
    /// - `self`: Record being configured.
    /// - `policy`: Retention category.
    /// - `strength`: Reference strength requested for exact inputs.
    ///
    /// # Returns
    /// The record with the requested retention metadata.
    #[must_use]
    pub const fn with_retention(
        mut self,
        policy: RetentionPolicy,
        strength: RetentionStrength,
    ) -> Self {
        self.retention = (policy, strength);
        self
    }

    /// Returns the exact derivation key.
    ///
    /// # Parameters
    /// - `self`: Record to inspect.
    ///
    /// # Returns
    /// The immutable derivation key.
    #[must_use]
    pub const fn key(&self) -> &DerivationKey {
        &self.key
    }

    /// Returns the immutable artifact handle.
    ///
    /// # Parameters
    /// - `self`: Record to inspect.
    ///
    /// # Returns
    /// The exact result identity.
    #[must_use]
    pub const fn handle(&self) -> DerivedResultHandle {
        self.handle
    }

    /// Returns the structured attempt status.
    ///
    /// # Parameters
    /// - `self`: Record to inspect.
    ///
    /// # Returns
    /// Success, deterministic failure, transient failure, or cancellation.
    #[must_use]
    pub const fn status(&self) -> DerivedResultStatus {
        self.status
    }

    /// Returns optional external freshness observation metadata.
    ///
    /// # Parameters
    /// - `self`: Record to inspect.
    ///
    /// # Returns
    /// Observation when present.
    #[must_use]
    pub const fn freshness(&self) -> Option<&FreshnessObservation> {
        self.freshness.as_ref()
    }

    /// Returns an optional structured failure reason.
    ///
    /// # Parameters
    /// - `self`: Record to inspect.
    ///
    /// # Returns
    /// Failure category when present.
    #[must_use]
    pub const fn failure(&self) -> Option<&FailureReason> {
        self.failure.as_ref()
    }

    /// Returns independent retention category and reference strength.
    ///
    /// # Parameters
    /// - `self`: Record to inspect.
    ///
    /// # Returns
    /// Retention metadata.
    #[must_use]
    pub const fn retention(&self) -> (RetentionPolicy, RetentionStrength) {
        self.retention
    }
}
