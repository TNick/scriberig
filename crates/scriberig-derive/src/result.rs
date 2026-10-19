//! Defines immutable derived result records and structured attempt status.

use crate::dependency::DependencySet;
use crate::event::DerivedResultHandle;
use crate::freshness::FreshnessObservation;
use crate::key::DerivationKey;
use crate::retention::{RetentionPolicy, RetentionStrength};
use scriberig_core::{EntityRevisionRef, KnowledgeValue, Provenance, ProvenanceSource};
use scriberig_graph::{DerivedArtifact, DerivedStatus};

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
/// - `with_payload`: Attaches the derived value.
/// - `with_provenance`: Attaches producer and input provenance.
/// - `key`: Returns the exact derivation key.
/// - `handle`: Returns the immutable result handle.
/// - `payload`: Returns the optional derived value.
/// - `provenance`: Returns producer and input provenance.
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
/// - `payload`: Optional derived value.
/// - `provenance`: Producer and input provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct DerivedResultRecord {
    key: DerivationKey,
    handle: DerivedResultHandle,
    status: DerivedResultStatus,
    freshness: Option<FreshnessObservation>,
    failure: Option<FailureReason>,
    retention: (RetentionPolicy, RetentionStrength),
    payload: Option<KnowledgeValue>,
    provenance: Provenance,
}

// Construct and inspect immutable derivation outcomes.
impl DerivedResultRecord {
    /// Wraps a universal graph artifact with derivation identity and policy.
    ///
    /// - `key`: Exact derivation request identity.
    /// - `handle`: Immutable result identity.
    /// - `artifact`: Universal derived artifact and its provenance.
    ///
    /// A cache record retaining the artifact payload, provenance, and status.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn from_artifact(
        key: DerivationKey,
        handle: DerivedResultHandle,
        artifact: DerivedArtifact,
    ) -> Self {
        let status = match artifact.status {
            DerivedStatus::Complete => DerivedResultStatus::Succeeded,
            DerivedStatus::Failed => DerivedResultStatus::FailedDeterministic,
            DerivedStatus::Pending => DerivedResultStatus::Cancelled,
        };
        Self::new(key, handle, status)
            .with_payload_opt(artifact.result)
            .with_provenance(artifact.provenance)
    }

    /// Creates an immutable result record.
    ///
    /// - `key`: Exact derivation request identity.
    /// - `handle`: Immutable artifact identity.
    /// - `status`: Structured attempt outcome.
    ///
    /// A record with disposable retention and weak input references.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(
        key: DerivationKey,
        handle: DerivedResultHandle,
        status: DerivedResultStatus,
    ) -> Self {
        let profile = format!("{}/{}", key.profile.kind(), key.profile.version());
        let inputs = key
            .dependencies
            .as_slice()
            .iter()
            .filter_map(|dependency| match dependency {
                crate::Dependency::ExactRevision { entity, revision } => Some(EntityRevisionRef {
                    entity_id: *entity,
                    revision_id: *revision,
                }),
                _ => None,
            })
            .collect();
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
            payload: None,
            provenance: Provenance {
                source: ProvenanceSource::System,
                producer: None,
                producer_version: None,
                inputs,
                profile: Some(profile),
                note: None,
            },
        }
    }

    /// Associates an exact historical source observation with this record.
    ///
    /// - `self`: Record being configured.
    /// - `observation`: Immutable observation and freshness deadline.
    ///
    /// The record with observation metadata attached.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_freshness(mut self, observation: FreshnessObservation) -> Self {
        self.freshness = Some(observation);
        self
    }

    /// Attaches a machine-readable failure reason.
    ///
    /// - `self`: Record being configured.
    /// - `reason`: Structured failure category.
    ///
    /// The record with failure metadata attached.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_failure(mut self, reason: FailureReason) -> Self {
        self.failure = Some(reason);
        self
    }

    /// Sets retention category and canonical input reference strength.
    ///
    /// - `self`: Record being configured.
    /// - `policy`: Retention category.
    /// - `strength`: Reference strength requested for exact inputs.
    ///
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

    /// Attaches the value produced by the derivation.
    ///
    /// - `self`: Record being configured.
    /// - `payload`: Derived semantic value.
    ///
    /// The record with its derived payload attached.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_payload(mut self, payload: KnowledgeValue) -> Self {
        self.payload = Some(payload);
        self
    }

    /// Attaches an optional universal artifact payload.
    ///
    /// - `self`: Record being configured.
    /// - `payload`: Optional derived semantic value.
    ///
    /// The record with the payload attached when present.
    #[must_use]
    /// # Returns
    /// The operation result.
    fn with_payload_opt(mut self, payload: Option<KnowledgeValue>) -> Self {
        self.payload = payload;
        self
    }

    /// Attaches producer and exact-input provenance.
    ///
    /// - `self`: Record being configured.
    /// - `provenance`: Origin and input metadata.
    ///
    /// The record with provenance attached.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = provenance;
        self
    }

    /// Returns the exact derivation key.
    ///
    /// - `self`: Record to inspect.
    ///
    /// The immutable derivation key.
    #[must_use]
    pub const fn key(&self) -> &DerivationKey {
        &self.key
    }

    /// Returns the exact dependencies that contribute to this artifact.
    ///
    /// - `self`: Record to inspect.
    ///
    /// The normalized immutable dependency set.
    #[must_use]
    pub const fn dependencies(&self) -> &DependencySet {
        &self.key.dependencies
    }

    /// Returns the immutable artifact handle.
    ///
    /// - `self`: Record to inspect.
    ///
    /// The exact result identity.
    #[must_use]
    pub const fn handle(&self) -> DerivedResultHandle {
        self.handle
    }

    /// Returns the structured attempt status.
    ///
    /// - `self`: Record to inspect.
    ///
    /// Success, deterministic failure, transient failure, or cancellation.
    #[must_use]
    pub const fn status(&self) -> DerivedResultStatus {
        self.status
    }

    /// Returns optional external freshness observation metadata.
    ///
    /// - `self`: Record to inspect.
    ///
    /// Observation when present.
    #[must_use]
    pub const fn freshness(&self) -> Option<&FreshnessObservation> {
        self.freshness.as_ref()
    }

    /// Returns an optional structured failure reason.
    ///
    /// - `self`: Record to inspect.
    ///
    /// Failure category when present.
    #[must_use]
    pub const fn failure(&self) -> Option<&FailureReason> {
        self.failure.as_ref()
    }

    /// Returns independent retention category and reference strength.
    ///
    /// - `self`: Record to inspect.
    ///
    /// Retention metadata.
    #[must_use]
    pub const fn retention(&self) -> (RetentionPolicy, RetentionStrength) {
        self.retention
    }

    /// Returns the optional derived value.
    ///
    /// - `self`: Record to inspect.
    ///
    /// The retained payload when the derivation produced one.
    #[must_use]
    pub const fn payload(&self) -> Option<&KnowledgeValue> {
        self.payload.as_ref()
    }

    /// Returns producer and exact-input provenance.
    ///
    /// - `self`: Record to inspect.
    ///
    /// The immutable provenance metadata.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}
