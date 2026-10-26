//! Holds retained provider continuation handles for the lifetime of one process.

use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;

use rpi_ai::{Provider, ProviderContinuation};
use scriberig_core::EntityId;

use super::ContinuationBinding;

/// One retained handle with the exact execution identity that produced it.
///
/// # Public members
/// None.
///
/// # Private members
/// - `binding`: Execution identity the handle was produced under.
/// - `handle`: Opaque provider-owned process-local state.
#[derive(Debug)]
struct RetainedHandle {
    /// Execution identity the handle was produced under.
    binding: ContinuationBinding,
    /// Opaque provider-owned process-local state.
    handle: ProviderContinuation,
}

/// Completed attempt: the extracted handle and the identity it belongs to.
///
/// # Public members
/// - `binding`: Identity the extracted handle was produced under.
/// - `handle`: Opaque handle extracted from the completed provider response.
///
/// # Private members
/// None.
#[derive(Clone, Debug, PartialEq)]
pub struct CompletedContinuation {
    /// Identity the extracted handle was produced under.
    pub binding: ContinuationBinding,
    /// Opaque handle extracted from the completed provider response.
    pub handle: ProviderContinuation,
}

/// Terminal outcome of one attempted execution for one execution identity.
///
/// Only `Completed` may retain a handle. Every other variant clears the identity
/// so the next attempt seeds from canonical compiled `ScribeRig` context. The
/// completed payload is boxed so the common clearing variants stay small.
///
/// # Public members
/// - `Completed`: Completed attempt carrying its handle and binding.
/// - `NoHandle`: Completed attempt whose provider extracted no reusable handle.
/// - `Failed`: Failed attempt; any handle for the identity is dropped.
/// - `Cancelled`: Cancelled attempt; any handle for the identity is dropped.
///
/// # Private members
/// None.
#[derive(Clone, Debug, PartialEq)]
pub enum ContinuationOutcome {
    /// Completed attempt whose provider response produced a reusable handle.
    Completed(Box<CompletedContinuation>),
    /// Completed attempt whose provider response extracted no reusable handle.
    NoHandle,
    /// Failed attempt; any handle for the identity is dropped.
    Failed,
    /// Cancelled attempt; any handle for the identity is dropped.
    Cancelled,
}

/// Process-local owner of opaque provider continuation handles.
///
/// Handles are keyed by the caller's `ScribeRig` execution identity and are
/// consumed before each attempt, so a failed, cancelled, or divergent attempt
/// cannot leave a handle reusable. The map is never serialized: it exists only
/// for the lifetime of the owning process, and an empty owner is always a valid
/// state because canonical `ScribeRig` context can resume any branch.
///
/// # Public members
/// - `new`: Builds an empty owner.
/// - `take_if_current`: Consumes and returns a still-current handle.
/// - `settle`: Applies one attempt outcome to the retained handle.
/// - `discard`: Drops any handle for one execution identity.
/// - `branches`: Sorted execution identities currently holding a handle.
/// - `is_empty`: Whether no handle is retained.
///
/// # Private members
/// - `entries`: Retained handles keyed by execution identity.
/// - `remove`: Removes one retained handle.
#[derive(Default)]
pub struct EphemeralContinuationOwner {
    /// Retained handles keyed by the `ScribeRig` execution identity that owns them.
    entries: Mutex<HashMap<EntityId, RetainedHandle>>,
}

impl EphemeralContinuationOwner {
    /// Builds an owner with no retained handle.
    ///
    /// # Returns
    /// An empty process-local owner, which is also the post-restart state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes and returns the handle for `branch` while `binding` is still current.
    ///
    /// The entry is removed before it is validated, so an attempt that consumes a
    /// handle and then fails, is cancelled, or diverges leaves nothing behind.
    ///
    /// # Parameters
    /// - `provider`: Upstream provider selected for the attempt.
    /// - `branch`: `ScribeRig` execution identity owning the handle.
    /// - `binding`: Exact identity of the execution about to run.
    ///
    /// # Returns
    /// The opaque handle when the selected provider advertises continuation and
    /// accepts this binding's provider identity, the exact binding still matches
    /// the retained one, and the handle is scoped to the selected provider/model.
    /// `None` reseeds the attempt from canonical context.
    #[must_use]
    pub fn take_if_current(
        &self,
        provider: &dyn Provider,
        branch: EntityId,
        binding: &ContinuationBinding,
    ) -> Option<ProviderContinuation> {
        let RetainedHandle {
            binding: retained,
            handle,
        } = self.remove(branch)?;
        let current = provider.supports_response_continuation()
            && binding.provider_id == provider.id()
            && binding.matches(&retained)
            && handle.is_scoped_to(provider.id(), &binding.model.id);
        current.then_some(handle)
    }

    /// Applies one attempt outcome, retaining a handle only for a completed attempt.
    ///
    /// # Parameters
    /// - `provider`: Upstream provider that ran the attempt.
    /// - `branch`: `ScribeRig` execution identity the attempt belonged to.
    /// - `outcome`: Terminal outcome, including any extracted handle and the
    ///   identity it was extracted under.
    ///
    /// # Returns
    /// `true` when a handle was retained for `branch`. A failure, cancellation,
    /// missing handle, unsupported provider, binding/provider mismatch, or
    /// out-of-scope handle returns `false` and leaves `branch` with no retained
    /// handle.
    pub fn settle(
        &self,
        provider: &dyn Provider,
        branch: EntityId,
        outcome: ContinuationOutcome,
    ) -> bool {
        let ContinuationOutcome::Completed(completed) = outcome else {
            self.remove(branch);
            return false;
        };
        let CompletedContinuation { binding, handle } = *completed;
        if !provider.supports_response_continuation()
            || binding.provider_id != provider.id()
            || !handle.is_scoped_to(provider.id(), &binding.model.id)
        {
            self.remove(branch);
            return false;
        }
        // A poisoned guard makes every read path fail closed, so no retained
        // handle can be handed to a provider attempt; report nothing retained.
        let Some(mut entries) = self.entries.lock().ok() else {
            return false;
        };
        entries.insert(branch, RetainedHandle { binding, handle });
        true
    }

    /// Drops any retained handle for one `ScribeRig` execution identity.
    ///
    /// # Parameters
    /// - `branch`: Execution identity whose handle is discarded.
    ///
    /// # Returns
    /// `true` when a handle was removed.
    pub fn discard(&self, branch: EntityId) -> bool {
        self.remove(branch).is_some()
    }

    /// Returns the execution identities currently holding a handle.
    ///
    /// # Returns
    /// Sorted `ScribeRig` execution identities with a retained handle.
    #[must_use]
    pub fn branches(&self) -> Vec<EntityId> {
        let mut branches = self.entries.lock().map_or_else(
            |_| Vec::new(),
            |entries| entries.keys().copied().collect::<Vec<_>>(),
        );
        branches.sort();
        branches
    }

    /// Reports whether no handle is retained.
    ///
    /// # Returns
    /// `true` when the next attempt must seed from canonical context.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries
            .lock()
            .map_or_else(|_| true, |entries| entries.is_empty())
    }

    /// Removes one retained handle, failing closed when the guard is unusable.
    ///
    /// # Parameters
    /// - `branch`: Execution identity whose handle is removed.
    ///
    /// # Returns
    /// The removed handle and its binding, when one was retained.
    fn remove(&self, branch: EntityId) -> Option<RetainedHandle> {
        self.entries.lock().ok()?.remove(&branch)
    }
}

impl fmt::Debug for EphemeralContinuationOwner {
    /// Reports how many handles are retained while keeping provider state opaque.
    ///
    /// # Parameters
    /// - `self`: Owner being formatted.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatting success, or the formatter error when output cannot be written.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let retained = self.entries.lock().map_or(0, |entries| entries.len());
        formatter
            .debug_struct("EphemeralContinuationOwner")
            .field("retained", &retained)
            .field("handles", &"[opaque]")
            .finish()
    }
}
