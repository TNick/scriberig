//! Defines a backend-independent immutable derived-result repository contract.

use crate::dependency::{Dependency, DependencySet};
use crate::event::DerivedResultHandle;
use crate::key::DerivationKey;
use crate::result::DerivedResultRecord;
use scriberig_core::{EntityId, RevisionId};

/// Reports an attempted overwrite of an immutable result identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepositoryError {
    /// The handle already identifies a different immutable record.
    ConflictingResult {
        /// Immutable handle already associated with different record data.
        handle: DerivedResultHandle,
    },
}

/// Stores and discovers immutable derived-result records.
///
/// Implementations may deduplicate identical inserts and must permit distinct
/// handles for one key. Removal is explicit and applies only to disposable
/// derived state; it has no canonical snapshot semantics.
pub trait DerivedRepository {
    /// Inserts an immutable result without replacing historical records.
    ///
    /// - `self`: Repository to update.
    /// - `record`: Immutable derivation outcome.
    ///
    /// Success when inserted or already present identically.
    ///
    /// Returns a conflict if the handle already names different record data.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn insert(&mut self, record: DerivedResultRecord) -> Result<(), RepositoryError>;

    /// Retrieves an exact immutable record by its stable handle.
    ///
    /// - `self`: Repository to inspect.
    /// - `handle`: Exact result identity.
    ///
    /// The historical record when retained.
    ///
    /// This lookup is infallible and returns `None` when absent.
    ///
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn get(&self, handle: DerivedResultHandle) -> Option<&DerivedResultRecord>;

    /// Finds every retained record for one exact derivation key.
    ///
    /// - `self`: Repository to inspect.
    /// - `key`: Exact profile and dependency identity.
    ///
    /// Matching immutable records.
    ///
    /// This lookup is infallible and returns an empty list when no key matches.
    ///
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn by_key(&self, key: &DerivationKey) -> Vec<&DerivedResultRecord>;

    /// Finds result handles that declare one exact dependency.
    ///
    /// - `self`: Repository to inspect.
    /// - `dependency`: Exact dependency to reverse-index.
    ///
    /// Stable handles of results depending on the requested value.
    ///
    /// This lookup is infallible and returns an empty list when unmatched.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn by_dependency(&self, dependency: &Dependency) -> Vec<DerivedResultHandle>;

    /// Returns the normalized dependencies recorded for one result.
    ///
    /// - `self`: Repository to inspect.
    /// - `handle`: Exact result identity.
    ///
    /// The immutable dependency set when the result is retained.
    ///
    /// This lookup is infallible and returns `None` when absent.
    /// # Returns
    /// The operation result.
    fn dependencies(&self, handle: DerivedResultHandle) -> Option<&DependencySet>;

    /// Finds results associated with an exact entity revision for discovery.
    ///
    /// - `self`: Repository to inspect.
    /// - `entity`: Exact entity identity.
    /// - `revision`: Exact revision identity.
    ///
    /// Stable handles associated with the exact entity revision.
    ///
    /// This lookup is infallible and returns an empty list when unmatched.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn by_entity_revision(
        &self,
        entity: EntityId,
        revision: RevisionId,
    ) -> Vec<DerivedResultHandle>;

    /// Removes one record only when its retention category is disposable.
    ///
    /// - `self`: Repository to update.
    /// - `handle`: Exact result identity.
    ///
    /// Whether a disposable record was removed.
    /// # Returns
    /// The operation result.
    fn remove_disposable(&mut self, handle: DerivedResultHandle) -> bool;
}
