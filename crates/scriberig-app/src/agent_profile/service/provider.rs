//! Exposes profile reads and mutations through the shared provider contract.

use scriberig_core::SnapshotId;
use scriberig_store::{RevisionStore, RootStore, SnapshotStore};

use super::operations::AgentProfileService;
use super::{AgentProfileError, AgentProfileStoreResult, ProfileCandidate};
use crate::{AgentProfileDescriptor, ApplicationContext};

/// Resolves and mutates caller-authorized profiles through a shared provider.
pub trait AgentProfileProvider: Send + Sync {
    /// Returns the current immutable profile snapshot, or the empty snapshot before bootstrap.
    ///
    /// # Returns
    /// The exact snapshot callers must use as a candidate base.
    ///
    /// # Errors
    /// Returns a storage error when the root cannot be read.
    fn current_snapshot(&self) -> Result<SnapshotId, AgentProfileError>;

    /// Reads the profile revision currently promoted on the provider's semantic root.
    ///
    /// # Parameters
    /// - `context`: Trusted caller capabilities.
    /// - `identity`: Stable profile identity.
    ///
    /// # Returns
    /// The exact current profile, or `None` before that identity is stored.
    ///
    /// # Errors
    /// Returns authorization, storage, or decoding errors.
    fn resolve_current_profile(
        &self,
        context: &ApplicationContext,
        identity: &str,
    ) -> Result<Option<AgentProfileDescriptor>, AgentProfileError>;

    /// Proposes a profile through the provider's universal candidate store.
    ///
    /// # Parameters
    /// - `context`: Trusted caller capabilities.
    /// - `base`: Exact current profile snapshot.
    /// - `profile`: Desired semantic profile descriptor.
    ///
    /// # Returns
    /// An immutable candidate bound to its exact base snapshot.
    ///
    /// # Errors
    /// Returns authorization, validation, storage, or unsupported-operation errors.
    fn propose_profile(
        &self,
        context: &ApplicationContext,
        base: SnapshotId,
        profile: AgentProfileDescriptor,
    ) -> Result<ProfileCandidate, AgentProfileError>;

    /// Promotes an authorized profile candidate through root compare-and-swap.
    ///
    /// # Parameters
    /// - `context`: Trusted caller capabilities.
    /// - `candidate`: Exact candidate returned by this provider.
    ///
    /// # Returns
    /// Whether the candidate became current or lost a concurrent root update.
    ///
    /// # Errors
    /// Returns authorization, ancestry, storage, or unsupported-operation errors.
    fn promote_candidate(
        &self,
        context: &ApplicationContext,
        candidate: &ProfileCandidate,
    ) -> Result<AgentProfileStoreResult, AgentProfileError>;
}

impl<S> AgentProfileProvider for AgentProfileService<S>
where
    S: RevisionStore + SnapshotStore + RootStore + Send + Sync,
{
    /// Reads the profile root snapshot through the shared provider contract.
    ///
    /// # Parameters
    /// - `self`: Shared profile provider.
    ///
    /// # Returns
    /// The exact current snapshot or the deterministic empty snapshot.
    ///
    /// # Errors
    /// Returns a storage error when reading the root fails.
    fn current_snapshot(&self) -> Result<SnapshotId, AgentProfileError> {
        AgentProfileService::current_snapshot(self)
    }

    /// Reads the exact current profile through this provider's semantic root.
    ///
    /// # Parameters
    /// - `self`: Shared profile provider.
    /// - `context`: Trusted caller capabilities.
    /// - `identity`: Stable profile identity.
    ///
    /// # Returns
    /// The exact current profile, or `None` before that identity is stored.
    ///
    /// # Errors
    /// Returns authorization, storage, or decoding errors.
    fn resolve_current_profile(
        &self,
        context: &ApplicationContext,
        identity: &str,
    ) -> Result<Option<AgentProfileDescriptor>, AgentProfileError> {
        AgentProfileService::resolve_current_profile(self, context, identity)
    }

    /// Proposes an immutable profile through the shared application provider.
    ///
    /// # Parameters
    /// - `self`: Shared profile provider.
    /// - `context`: Trusted caller capabilities.
    /// - `base`: Exact current profile snapshot.
    /// - `profile`: Desired semantic profile descriptor.
    ///
    /// # Returns
    /// An immutable candidate bound to its exact base snapshot.
    ///
    /// # Errors
    /// Returns authorization, validation, storage, or unsupported-operation errors.
    fn propose_profile(
        &self,
        context: &ApplicationContext,
        base: SnapshotId,
        profile: AgentProfileDescriptor,
    ) -> Result<ProfileCandidate, AgentProfileError> {
        AgentProfileService::propose_profile(self, context, base, profile)
    }

    /// Promotes one candidate through the shared application provider.
    ///
    /// # Parameters
    /// - `self`: Shared profile provider.
    /// - `context`: Trusted caller capabilities.
    /// - `candidate`: Exact candidate returned by this provider.
    ///
    /// # Returns
    /// Whether the candidate became current or lost a concurrent root update.
    ///
    /// # Errors
    /// Returns authorization, ancestry, storage, or unsupported-operation errors.
    fn promote_candidate(
        &self,
        context: &ApplicationContext,
        candidate: &ProfileCandidate,
    ) -> Result<AgentProfileStoreResult, AgentProfileError> {
        AgentProfileService::promote_candidate(self, context, candidate)
    }
}
