//! Defines a backend-independent immutable derived-result repository contract.

use crate::event::DerivedResultHandle;
use crate::key::DerivationKey;
use crate::result::DerivedResultRecord;

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
    /// # Parameters
    /// - `self`: Repository to update.
    /// - `record`: Immutable derivation outcome.
    ///
    /// # Returns
    /// Success when inserted or already present identically.
    ///
    /// # Errors
    /// Returns a conflict if the handle already names different record data.
    fn insert(&mut self, record: DerivedResultRecord) -> Result<(), RepositoryError>;

    /// Retrieves an exact immutable record by its stable handle.
    ///
    /// # Parameters
    /// - `self`: Repository to inspect.
    /// - `handle`: Exact result identity.
    ///
    /// # Returns
    /// The historical record when retained.
    ///
    /// # Errors
    /// This lookup is infallible and returns `None` when absent.
    ///
    /// # Errors
    /// This lookup is infallible and returns `None` when the record is absent.
    fn get(&self, handle: DerivedResultHandle) -> Option<&DerivedResultRecord>;

    /// Finds every retained record for one exact derivation key.
    ///
    /// # Parameters
    /// - `self`: Repository to inspect.
    /// - `key`: Exact profile and dependency identity.
    ///
    /// # Returns
    /// Matching immutable records.
    ///
    /// # Errors
    /// This lookup is infallible and returns an empty list when no key matches.
    ///
    /// # Errors
    /// This lookup is infallible and returns an empty list when no key matches.
    fn by_key(&self, key: &DerivationKey) -> Vec<&DerivedResultRecord>;

    /// Removes one record only when its retention category is disposable.
    ///
    /// # Parameters
    /// - `self`: Repository to update.
    /// - `handle`: Exact result identity.
    ///
    /// # Returns
    /// Whether a disposable record was removed.
    fn remove_disposable(&mut self, handle: DerivedResultHandle) -> bool;
}
