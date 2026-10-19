//! Provides fake-domain identifiers, keys, records, and an in-memory repository.

use scriberig_core::{EntityId, RevisionId};
use scriberig_derive::{
    Dependency, DependencySet, DerivationKey, DerivedRepository, DerivedResultHandle,
    DerivedResultRecord, DerivedResultStatus, RepositoryError, RetentionPolicy,
};
use std::collections::BTreeMap;

/// Stores immutable fake-domain results for integration scenarios.
///
/// # Public members
/// - None.
///
/// # Private members
/// - `records`: Exact result handles mapped to immutable records.
#[derive(Default)]
pub(super) struct TestRepository {
    records: BTreeMap<DerivedResultHandle, DerivedResultRecord>,
}

// Implements derived-only in-memory repository behavior for tests.
impl DerivedRepository for TestRepository {
    /// Inserts immutable records and deduplicates identical handles.
    ///
    /// - `self`: Repository to update.
    /// - `record`: Immutable fake-domain result.
    ///
    /// Success when inserted or already present identically.
    ///
    /// Returns a conflict when a handle is reused for different data.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn insert(&mut self, record: DerivedResultRecord) -> Result<(), RepositoryError> {
        let handle = record.handle();
        match self.records.get(&handle) {
            Some(existing) if existing != &record => {
                Err(RepositoryError::ConflictingResult { handle })
            }
            Some(_) => Ok(()),
            None => {
                self.records.insert(handle, record);
                Ok(())
            }
        }
    }

    /// Retrieves one exact historical record.
    ///
    /// - `self`: Repository to inspect.
    /// - `handle`: Exact result identity.
    ///
    /// Retained record when present.
    ///
    /// This lookup is infallible and returns `None` when absent.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn get(&self, handle: DerivedResultHandle) -> Option<&DerivedResultRecord> {
        self.records.get(&handle)
    }

    /// Finds records for one exact request key.
    ///
    /// - `self`: Repository to inspect.
    /// - `key`: Exact derivation request identity.
    ///
    /// Matching immutable records in handle order.
    ///
    /// This lookup is infallible and returns an empty list when no key matches.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn by_key(&self, key: &DerivationKey) -> Vec<&DerivedResultRecord> {
        self.records
            .values()
            .filter(|record| record.key() == key)
            .collect()
    }

    /// Finds handles that depend on one exact dependency.
    ///
    /// - `dependency`: Exact dependency to find.
    ///
    /// Matching immutable handles.
    ///
    /// This lookup is infallible.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn by_dependency(&self, dependency: &Dependency) -> Vec<DerivedResultHandle> {
        self.records
            .values()
            .filter(|record| record.key().dependencies.as_slice().contains(dependency))
            .map(DerivedResultRecord::handle)
            .collect()
    }

    /// Returns the exact dependencies recorded for a result.
    ///
    /// - `handle`: Exact result handle.
    ///
    /// The normalized dependencies, when retained.
    /// # Returns
    /// The operation result.
    fn dependencies(&self, handle: DerivedResultHandle) -> Option<&DependencySet> {
        self.records
            .get(&handle)
            .map(|record| &record.key().dependencies)
    }

    /// Finds handles associated with an exact entity revision.
    ///
    /// - `entity`: Exact entity identity.
    /// - `revision`: Exact revision identity.
    ///
    /// Matching immutable handles.
    ///
    /// This lookup is infallible.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn by_entity_revision(
        &self,
        entity: EntityId,
        revision: RevisionId,
    ) -> Vec<DerivedResultHandle> {
        self.by_dependency(&Dependency::ExactRevision { entity, revision })
    }

    /// Removes a record only when its retention category is disposable.
    ///
    /// - `self`: Repository to update.
    /// - `handle`: Exact result identity.
    ///
    /// `true` when a disposable record was removed.
    /// # Returns
    /// The operation result.
    fn remove_disposable(&mut self, handle: DerivedResultHandle) -> bool {
        let disposable = self
            .records
            .get(&handle)
            .is_some_and(|record| record.retention().0 == RetentionPolicy::Disposable);
        disposable && self.records.remove(&handle).is_some()
    }
}

/// Creates a fake universal entity identity.
///
/// - `value`: Byte repeated in the fixed-width test identifier.
///
/// A test entity ID.
///
/// Construction from fixed-width bytes is infallible.
/// # Returns
/// The operation result.
pub fn entity(value: u8) -> EntityId {
    EntityId::from_bytes([value; 16])
}

/// Creates a fake immutable revision identity.
///
/// - `value`: Byte repeated in the fixed-width test identifier.
///
/// A test revision ID.
///
/// Construction from fixed-width bytes is infallible.
/// # Returns
/// The operation result.
pub fn revision(value: u8) -> RevisionId {
    RevisionId::new([value; 32])
}

/// Creates a result handle from a fake immutable artifact ID.
///
/// - `value`: Byte repeated in the artifact identity.
///
/// Exact derived result handle.
///
/// Handle construction is infallible.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn handle(value: u8) -> DerivedResultHandle {
    DerivedResultHandle {
        artifact_id: revision(value),
    }
}

/// Builds an exact profile and normalized dependency key.
///
/// - `name`: Fake derivation profile kind.
/// - `dependencies`: Exact declared dependencies.
///
/// Deterministic derivation request key.
///
/// Key construction is infallible for already formed identifiers.
/// # Returns
/// The operation result.
pub fn key(name: &str, dependencies: Vec<Dependency>) -> DerivationKey {
    DerivationKey::new(
        scriberig_derive::DerivationProfile::new(name, "v1"),
        DependencySet::new(dependencies),
    )
}

/// Creates a successful immutable record for an exact key.
///
/// - `request`: Exact derivation key.
/// - `artifact`: Stable artifact identity byte.
///
/// Successful derived result record.
///
/// Record construction is infallible for exact keys and handles.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn successful(request: DerivationKey, artifact: u8) -> DerivedResultRecord {
    DerivedResultRecord::new(request, handle(artifact), DerivedResultStatus::Succeeded)
}
