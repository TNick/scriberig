//! Implements authorized profile reads, candidate creation, and promotion.

use std::str::FromStr;

use scriberig_core::{EncodingVersion, RevisionId, RootName, SnapshotId};
use scriberig_store::{
    CasOutcome, Revision, RevisionStore, RootStore, SnapshotChange, SnapshotStore,
};

use super::model::{
    AgentProfileError, AgentProfileStoreResult, ProfileCandidate, ProfileInspection,
};
use super::storage::{
    StoredComponent, StoredProfile, component_revision_entity, decode_payload, encode_payload,
    identity_entity, object_type, permits, store_error, validate_profile,
};
use super::{COMPONENT_TYPE, PROFILE_TYPE};
use crate::{AgentProfileDescriptor, ApplicationContext, CapabilityAction};

#[path = "operations/reads.rs"]
mod reads;

/// Shared application operations for semantic profiles and context components.
///
/// Profiles and components are persisted as separate universal immutable
/// revisions. Candidate snapshots use the common snapshot/change model, and
/// accepted candidates advance one named root using compare-and-swap.
///
/// # Public members
/// - `new`: Constructs the shared operations over universal stores.
/// - `current_snapshot`: Reads the current profile snapshot.
/// - `list_profiles`: Lists visible profiles at an exact snapshot.
/// - `inspect_profile`: Resolves a profile and its exact component revisions.
/// - `propose_profile`: Creates an authorized immutable candidate snapshot.
/// - `promote_candidate`: Promotes an authorized candidate with root CAS.
///
/// # Private members
/// - `store`: Universal immutable revision, snapshot, and root backend.
/// - `root`: Named root holding the accepted profile catalog.
pub struct AgentProfileService<S> {
    store: S,
    root: RootName,
}

impl<S> AgentProfileService<S> {
    /// Creates profile operations for one named semantic root.
    ///
    /// # Parameters
    /// - `store`: Universal revision, snapshot, and root backend.
    /// - `root`: Named root holding accepted profile state.
    ///
    /// # Returns
    /// A profile service over the supplied shared backend.
    #[must_use]
    pub const fn new(store: S, root: RootName) -> Self {
        Self { store, root }
    }
}

impl<S> AgentProfileService<S>
where
    S: RevisionStore + SnapshotStore + RootStore,
{
    /// Reads one exact profile revision and resolves its component revisions.
    ///
    /// # Parameters
    /// - `self`: Profile service to query.
    /// - `context`: Trusted caller capabilities.
    /// - `snapshot`: Exact snapshot containing the profile revision.
    /// - `identity`: Stable profile identity.
    /// - `revision`: Exact requested profile revision.
    ///
    /// # Returns
    /// The profile descriptor and its containing snapshot.
    ///
    /// # Errors
    /// Returns authorization, not-found, storage, or decoding errors.
    pub fn inspect_profile(
        &self,
        context: &ApplicationContext,
        snapshot: SnapshotId,
        identity: &str,
        revision: RevisionId,
    ) -> Result<ProfileInspection, AgentProfileError> {
        let entity = identity_entity(PROFILE_TYPE, identity)?;
        if !permits(context, &CapabilityAction::Inspect, entity, snapshot) {
            return Err(AgentProfileError::Unauthorized {
                action: "inspect".to_owned(),
                profile: identity.to_owned(),
            });
        }
        if self.store.resolve(snapshot, entity).map_err(store_error)? != Some(revision) {
            return Err(AgentProfileError::NotFound {
                profile: identity.to_owned(),
                snapshot,
            });
        }
        let value = self
            .store
            .get(revision)
            .map_err(store_error)?
            .ok_or_else(|| AgentProfileError::NotFound {
                profile: identity.to_owned(),
                snapshot,
            })?;
        if value.object_type().as_str() != PROFILE_TYPE {
            return Err(AgentProfileError::Invalid(
                "revision is not an agent profile".to_owned(),
            ));
        }
        let stored: StoredProfile = decode_payload(value.payload())?;
        if stored.identity != identity {
            return Err(AgentProfileError::Invalid(
                "stored profile identity does not match the requested identity".to_owned(),
            ));
        }
        self.expand_profile(snapshot, stored, value.id())
    }

    /// Resolves the current exact profile revision from this service's root.
    ///
    /// # Parameters
    /// - `self`: Profile service to query.
    /// - `context`: Trusted caller capabilities.
    /// - `identity`: Stable profile identity.
    ///
    /// # Returns
    /// The current profile descriptor, or `None` when it has not been stored.
    ///
    /// # Errors
    /// Returns authorization, storage, or decoding errors.
    pub fn resolve_current_profile(
        &self,
        context: &ApplicationContext,
        identity: &str,
    ) -> Result<Option<AgentProfileDescriptor>, AgentProfileError> {
        let snapshot = self.current_snapshot()?;
        let entity = identity_entity(PROFILE_TYPE, identity)?;
        let Some(revision) = self.store.resolve(snapshot, entity).map_err(store_error)? else {
            return Ok(None);
        };
        self.inspect_profile(context, snapshot, identity, revision)
            .map(|inspection| Some(inspection.profile))
    }

    /// Proposes a profile edit as a candidate snapshot without changing the accepted root.
    ///
    /// # Parameters
    /// - `self`: Profile service to use.
    /// - `context`: Trusted caller capabilities.
    /// - `base`: Exact current snapshot used as the candidate base.
    /// - `profile`: Desired profile descriptor; supplied revision strings are ignored.
    ///
    /// # Returns
    /// A candidate bound to its exact base, snapshot, identity, and revision.
    ///
    /// # Errors
    /// Returns authorization, stale-base, validation, or storage errors.
    pub fn propose_profile(
        &self,
        context: &ApplicationContext,
        base: SnapshotId,
        mut profile: AgentProfileDescriptor,
    ) -> Result<ProfileCandidate, AgentProfileError> {
        let actual = self.store.read_root(&self.root);
        let effective_base = actual.unwrap_or_else(|| scriberig_store::Snapshot::empty().id());
        if effective_base != base {
            return Err(AgentProfileError::StaleBase {
                expected: base,
                actual,
            });
        }
        validate_profile(&profile)?;
        let profile_entity = identity_entity(PROFILE_TYPE, &profile.reference.identity)?;
        if !permits(context, &CapabilityAction::Propose, profile_entity, base) {
            return Err(AgentProfileError::Unauthorized {
                action: "propose".to_owned(),
                profile: profile.reference.identity,
            });
        }
        let mut changes = Vec::new();
        for component in &mut profile.components {
            let stored_component = StoredComponent::from(&*component);
            let payload = encode_payload(&stored_component)?;
            let kind = object_type(COMPONENT_TYPE)?;
            let revision =
                Revision::new(EncodingVersion::V1, kind, payload).map_err(store_error)?;
            component.reference.revision = revision.id().to_string();
            let component_entity =
                component_revision_entity(&component.reference.identity, revision.id())?;
            self.store.put(revision.clone()).map_err(store_error)?;
            changes.push(SnapshotChange::Set(component_entity, revision.id()));
        }
        let stored_profile = StoredProfile::from(&profile);
        let payload = encode_payload(&stored_profile)?;
        let profile_revision =
            Revision::new(EncodingVersion::V1, object_type(PROFILE_TYPE)?, payload)
                .map_err(store_error)?;
        profile.reference.revision = profile_revision.id().to_string();
        self.store
            .put(profile_revision.clone())
            .map_err(store_error)?;
        changes.push(SnapshotChange::Set(profile_entity, profile_revision.id()));
        let snapshot = self
            .store
            .apply_changes(base, &changes)
            .map_err(store_error)?;
        Ok(ProfileCandidate {
            base,
            expected_root: actual,
            snapshot,
            profile: profile.reference.identity,
            revision: profile_revision.id(),
        })
    }

    /// Promotes a proposed profile snapshot using authorization and root compare-and-swap.
    ///
    /// # Parameters
    /// - `self`: Profile service to use.
    /// - `context`: Trusted caller capabilities.
    /// - `candidate`: Candidate returned by `propose_profile`.
    ///
    /// # Returns
    /// Whether the root was updated or had already moved.
    ///
    /// # Errors
    /// Returns authorization, ancestry, storage, or root backend errors.
    pub fn promote_candidate(
        &self,
        context: &ApplicationContext,
        candidate: &ProfileCandidate,
    ) -> Result<AgentProfileStoreResult, AgentProfileError> {
        let entity = identity_entity(PROFILE_TYPE, &candidate.profile)?;
        if !permits(context, &CapabilityAction::Promote, entity, candidate.base) {
            return Err(AgentProfileError::Unauthorized {
                action: "promote".to_owned(),
                profile: candidate.profile.clone(),
            });
        }
        let ancestors = self
            .store
            .ancestors(candidate.snapshot)
            .map_err(store_error)?;
        if !ancestors.contains(&candidate.base) {
            return Err(AgentProfileError::Invalid(
                "candidate does not descend from its exact base snapshot".to_owned(),
            ));
        }
        let actual = self.store.read_root(&self.root);
        if actual != candidate.expected_root {
            return Ok(AgentProfileStoreResult::Stale { actual });
        }
        match self
            .store
            .compare_and_swap(
                self.root.clone(),
                candidate.expected_root,
                candidate.snapshot,
            )
            .map_err(store_error)?
        {
            CasOutcome::Updated => Ok(AgentProfileStoreResult::Promoted {
                snapshot: candidate.snapshot,
            }),
            CasOutcome::Stale { actual } => Ok(AgentProfileStoreResult::Stale { actual }),
        }
    }

    /// Expands stored profile references into exact component descriptors.
    ///
    /// # Parameters
    /// - `self`: Profile service reading canonical storage.
    /// - `snapshot`: Exact snapshot for profile and component resolution.
    /// - `stored`: Stored profile metadata.
    /// - `revision`: Exact profile revision.
    ///
    /// # Returns
    /// The complete profile inspection.
    ///
    /// # Errors
    /// Returns not-found, storage, or decoding errors.
    fn expand_profile(
        &self,
        snapshot: SnapshotId,
        stored: StoredProfile,
        revision: RevisionId,
    ) -> Result<ProfileInspection, AgentProfileError> {
        let mut components = Vec::with_capacity(stored.components.len());
        for reference in &stored.components {
            let expected = RevisionId::from_str(&reference.revision)
                .map_err(|error| AgentProfileError::Encoding(error.to_string()))?;
            let entity = component_revision_entity(&reference.identity, expected)?;
            if self.store.resolve(snapshot, entity).map_err(store_error)? != Some(expected) {
                return Err(AgentProfileError::NotFound {
                    profile: stored.identity.clone(),
                    snapshot,
                });
            }
            let value = self
                .store
                .get(expected)
                .map_err(store_error)?
                .ok_or_else(|| AgentProfileError::NotFound {
                    profile: stored.identity.clone(),
                    snapshot,
                })?;
            if value.object_type().as_str() != COMPONENT_TYPE {
                return Err(AgentProfileError::Invalid(
                    "component reference resolves to a different semantic type".to_owned(),
                ));
            }
            let component: StoredComponent = decode_payload(value.payload())?;
            if component.identity != reference.identity || value.id() != expected {
                return Err(AgentProfileError::Invalid(
                    "component identity or revision does not match its profile reference"
                        .to_owned(),
                ));
            }
            components.push(component.into_descriptor(reference.clone()));
        }
        let profile = AgentProfileDescriptor {
            reference: crate::AgentProfileRevisionRef {
                identity: stored.identity,
                revision: revision.to_string(),
            },
            name: stored.name,
            components,
            preferred_execution: stored.preferred_execution,
        };
        Ok(ProfileInspection { profile, snapshot })
    }
}
