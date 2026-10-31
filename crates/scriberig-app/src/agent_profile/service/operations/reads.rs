//! Implements snapshot-bound profile listing and resolution operations.

use scriberig_core::SnapshotId;
use scriberig_store::{RevisionStore, RootStore, SnapshotStore};

use super::super::model::{AgentProfileError, ProfileInspection};
use super::super::storage::{StoredProfile, decode_payload, identity_entity, permits, store_error};
use super::AgentProfileService;
use super::PROFILE_TYPE;
use crate::{ApplicationContext, CapabilityAction};

impl<S> AgentProfileService<S>
where
    S: RevisionStore + SnapshotStore + RootStore,
{
    /// Returns the current profile snapshot, using the universal empty snapshot before bootstrap.
    ///
    /// # Parameters
    /// - `self`: Profile service to query.
    ///
    /// # Returns
    /// The accepted snapshot or the deterministic empty snapshot.
    ///
    /// # Errors
    /// Returns an error when the root backend fails.
    pub fn current_snapshot(&self) -> Result<SnapshotId, AgentProfileError> {
        Ok(self
            .store
            .read_root(&self.root)
            .unwrap_or_else(|| scriberig_store::Snapshot::empty().id()))
    }

    /// Lists profiles visible to the caller in one exact snapshot.
    ///
    /// # Parameters
    /// - `self`: Profile service to query.
    /// - `context`: Trusted caller capabilities.
    /// - `snapshot`: Exact immutable snapshot to inspect.
    ///
    /// # Returns
    /// Visible profile descriptors with exact component revisions.
    ///
    /// # Errors
    /// Returns an error when storage is unavailable or a visible profile is malformed.
    pub fn list_profiles(
        &self,
        context: &ApplicationContext,
        snapshot: SnapshotId,
    ) -> Result<Vec<ProfileInspection>, AgentProfileError> {
        let mut profiles = Vec::new();
        for (entity, revision_id) in self.store.iter(snapshot).map_err(store_error)? {
            let revision = self.store.get(revision_id).map_err(store_error)?;
            let Some(revision) = revision else { continue };
            if revision.object_type().as_str() != PROFILE_TYPE {
                continue;
            }
            if !permits(context, &CapabilityAction::Inspect, entity, snapshot) {
                continue;
            }
            let stored: StoredProfile = decode_payload(revision.payload())?;
            if identity_entity(PROFILE_TYPE, &stored.identity)? != entity {
                return Err(AgentProfileError::Invalid(
                    "stored profile identity does not match its semantic entity".to_owned(),
                ));
            }
            profiles.push(self.expand_profile(snapshot, stored, revision.id())?);
        }
        profiles.sort_by(|left, right| {
            left.profile
                .reference
                .identity
                .cmp(&right.profile.reference.identity)
        });
        Ok(profiles)
    }
}
