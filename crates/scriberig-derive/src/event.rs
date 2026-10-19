//! Defines derived-state lifecycle events distinct from canonical state events.

use crate::key::DerivationKey;
use crate::profile::DerivationProfile;
use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Identifies one immutable derived-result artifact without payload contents.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DerivedResultHandle {
    /// Content-addressed immutable result identity.
    pub artifact_id: RevisionId,
}

/// Provides machine-readable context for a failed or explained lifecycle event.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum EventReason {
    /// Derivation completed with a deterministic failure.
    DeterministicFailure,
    /// Derivation failed transiently and may be retried.
    TransientFailure,
    /// Work was cancelled or did not complete.
    Cancelled,
    /// The exact dependency state no longer matches the requested state.
    DependenciesChanged,
    /// A result was removed according to a retention decision.
    RetentionPolicy,
    /// An index materialization was refreshed for the indicated snapshot.
    IndexRefresh {
        /// Snapshot represented by the refreshed index.
        snapshot: SnapshotId,
    },
    /// An extensible machine-readable reason identifier.
    Extension(String),
}

/// Describes a derived result, selection, or index lifecycle transition.
///
/// These variants report analysis/cache/index availability. They never assert
/// that authoritative semantic state changed and never imply a snapshot
/// transition. Canonical snapshot events belong to a separate event stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DerivationEvent {
    /// A derivation attempt began for this exact profile and key.
    DerivationStarted {
        /// Exact derivation definition.
        profile: DerivationProfile,
        /// Exact request identity.
        key: DerivationKey,
        /// Optional subject entity for discovery and UI context.
        subject: Option<EntityId>,
    },
    /// An immutable derived result became available for this exact key.
    DerivedResultAvailable {
        /// Exact derivation definition.
        profile: DerivationProfile,
        /// Exact request identity.
        key: DerivationKey,
        /// Immutable result handle.
        result: DerivedResultHandle,
        /// Optional subject entity.
        subject: Option<EntityId>,
    },
    /// A derivation attempt failed with a structured reason.
    DerivationFailed {
        /// Exact derivation definition.
        profile: DerivationProfile,
        /// Exact request identity.
        key: DerivationKey,
        /// Structured failure classification.
        reason: EventReason,
        /// Optional subject entity.
        subject: Option<EntityId>,
    },
    /// A consumer's derived-result selection changed.
    DerivedResultSelected {
        /// Exact derivation definition.
        profile: DerivationProfile,
        /// Exact request identity.
        key: DerivationKey,
        /// Selected immutable result handle.
        result: DerivedResultHandle,
        /// Optional subject entity.
        subject: Option<EntityId>,
    },
    /// A retained derived result was evicted from derived storage.
    DerivedResultEvicted {
        /// Exact derivation definition.
        profile: DerivationProfile,
        /// Exact request identity.
        key: DerivationKey,
        /// Evicted immutable result handle.
        result: DerivedResultHandle,
        /// Structured eviction reason.
        reason: EventReason,
    },
    /// A result is known not to apply to this exact state anymore.
    DerivedResultInvalidatedForState {
        /// Exact derivation definition.
        profile: DerivationProfile,
        /// Exact request identity of the prior result.
        key: DerivationKey,
        /// Prior immutable result handle.
        result: DerivedResultHandle,
        /// Exact target snapshot for which it is invalid.
        target_snapshot: SnapshotId,
        /// Structured invalidation reason.
        reason: EventReason,
    },
    /// A derived index materialization was updated for a scope snapshot.
    IndexUpdated {
        /// Exact index derivation profile.
        profile: DerivationProfile,
        /// Exact request identity.
        key: DerivationKey,
        /// Snapshot represented by the index.
        snapshot: SnapshotId,
        /// Optional subject/scope entity associated with the index.
        subject: Option<EntityId>,
    },
}

/// Returns the lifecycle kind without converting it into a transport event.
///
/// - `event`: Event to classify.
///
/// Stable derived lifecycle category.
///
/// This classification does not represent a canonical snapshot event.
#[must_use]
pub const fn event_kind(event: &DerivationEvent) -> DerivationEventKind {
    match event {
        DerivationEvent::DerivationStarted { .. } => DerivationEventKind::DerivationStarted,
        DerivationEvent::DerivedResultAvailable { .. } => {
            DerivationEventKind::DerivedResultAvailable
        }
        DerivationEvent::DerivationFailed { .. } => DerivationEventKind::DerivationFailed,
        DerivationEvent::DerivedResultSelected { .. } => DerivationEventKind::DerivedResultSelected,
        DerivationEvent::DerivedResultEvicted { .. } => DerivationEventKind::DerivedResultEvicted,
        DerivationEvent::DerivedResultInvalidatedForState { .. } => {
            DerivationEventKind::DerivedResultInvalidatedForState
        }
        DerivationEvent::IndexUpdated { .. } => DerivationEventKind::IndexUpdated,
    }
}

/// Names one derived lifecycle event category.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DerivationEventKind {
    /// A derivation attempt began.
    DerivationStarted,
    /// A derived result became available.
    DerivedResultAvailable,
    /// A derivation attempt failed.
    DerivationFailed,
    /// A result was selected for use.
    DerivedResultSelected,
    /// A result was evicted.
    DerivedResultEvicted,
    /// A result was invalidated for an exact state.
    DerivedResultInvalidatedForState,
    /// A derived index was updated.
    IndexUpdated,
}

// Verify event construction and separation from canonical snapshot events.
#[cfg(test)]
mod tests {
    use super::{
        DerivationEvent, DerivationEventKind, DerivedResultHandle, EventReason, event_kind,
    };
    use crate::dependency::{Dependency, DependencySet};
    use crate::key::DerivationKey;
    use crate::profile::{DerivationProfile, DerivationSchedule};
    use scriberig_core::{EntityId, RevisionId, SnapshotId};

    /// Builds a stable test key using a universal exact entity revision.
    ///
    /// An exact test derivation key.
    /// # Returns
    /// The operation result.
    fn key() -> DerivationKey {
        let profile = DerivationProfile::new("test-summary", "v1");
        let dependencies = DependencySet::new(vec![Dependency::ExactRevision {
            entity: EntityId::from_bytes([1; 16]),
            revision: RevisionId::new([2; 32]),
        }]);
        DerivationKey::new(profile, dependencies)
    }

    /// Constructs each required lifecycle kind without a transport envelope.
    #[test]
    fn all_derived_event_kinds_are_constructible() {
        let profile = DerivationProfile::new("test-summary", "v1");
        let key = key();
        let result = DerivedResultHandle {
            artifact_id: RevisionId::new([3; 32]),
        };
        let subject = Some(EntityId::from_bytes([1; 16]));
        let snapshot = SnapshotId::new([4; 32]);
        let events = [
            DerivationEvent::DerivationStarted {
                profile: profile.clone(),
                key: key.clone(),
                subject,
            },
            DerivationEvent::DerivedResultAvailable {
                profile: profile.clone(),
                key: key.clone(),
                result,
                subject,
            },
            DerivationEvent::DerivationFailed {
                profile: profile.clone(),
                key: key.clone(),
                reason: EventReason::TransientFailure,
                subject,
            },
            DerivationEvent::DerivedResultSelected {
                profile: profile.clone(),
                key: key.clone(),
                result,
                subject,
            },
            DerivationEvent::DerivedResultEvicted {
                profile: profile.clone(),
                key: key.clone(),
                result,
                reason: EventReason::RetentionPolicy,
            },
            DerivationEvent::DerivedResultInvalidatedForState {
                profile: profile.clone(),
                key: key.clone(),
                result,
                target_snapshot: snapshot,
                reason: EventReason::DependenciesChanged,
            },
            DerivationEvent::IndexUpdated {
                profile,
                key,
                snapshot,
                subject,
            },
        ];

        assert_eq!(
            events.map(|event| event_kind(&event)),
            [
                DerivationEventKind::DerivationStarted,
                DerivationEventKind::DerivedResultAvailable,
                DerivationEventKind::DerivationFailed,
                DerivationEventKind::DerivedResultSelected,
                DerivationEventKind::DerivedResultEvicted,
                DerivationEventKind::DerivedResultInvalidatedForState,
                DerivationEventKind::IndexUpdated,
            ]
        );
    }

    /// Confirms result events carry the exact key and immutable result handle.
    #[test]
    fn result_events_carry_exact_key_and_result_handle() {
        let key = key();
        let result = DerivedResultHandle {
            artifact_id: RevisionId::new([7; 32]),
        };
        let event = DerivationEvent::DerivedResultAvailable {
            profile: key.profile.clone(),
            key: key.clone(),
            result,
            subject: None,
        };

        assert!(matches!(event, DerivationEvent::DerivedResultAvailable {
            key: event_key,
            result: event_result,
            ..
        } if event_key == key && event_result == result));
    }

    /// Documents by API shape that no canonical snapshot event exists here.
    #[test]
    fn event_vocabulary_contains_only_derived_lifecycle_kinds() {
        let kinds = [
            DerivationEventKind::DerivationStarted,
            DerivationEventKind::DerivedResultAvailable,
            DerivationEventKind::DerivationFailed,
            DerivationEventKind::DerivedResultSelected,
            DerivationEventKind::DerivedResultEvicted,
            DerivationEventKind::DerivedResultInvalidatedForState,
            DerivationEventKind::IndexUpdated,
        ];

        assert_eq!(kinds.len(), 7);
        assert_eq!(
            DerivationProfile::new("custom", "v1").schedule(),
            DerivationSchedule::Lazy
        );
    }
}
