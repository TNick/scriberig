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
    /// # Parameters
    /// - `self`: Repository to update.
    /// - `record`: Immutable fake-domain result.
    ///
    /// # Returns
    /// Success when inserted or already present identically.
    ///
    /// # Errors
    /// Returns a conflict when a handle is reused for different data.
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
    /// # Parameters
    /// - `self`: Repository to inspect.
    /// - `handle`: Exact result identity.
    ///
    /// # Returns
    /// Retained record when present.
    ///
    /// # Errors
    /// This lookup is infallible and returns `None` when absent.
    fn get(&self, handle: DerivedResultHandle) -> Option<&DerivedResultRecord> {
        self.records.get(&handle)
    }

    /// Finds records for one exact request key.
    ///
    /// # Parameters
    /// - `self`: Repository to inspect.
    /// - `key`: Exact derivation request identity.
    ///
    /// # Returns
    /// Matching immutable records in handle order.
    ///
    /// # Errors
    /// This lookup is infallible and returns an empty list when no key matches.
    fn by_key(&self, key: &DerivationKey) -> Vec<&DerivedResultRecord> {
        self.records
            .values()
            .filter(|record| record.key() == key)
            .collect()
    }

    /// Finds handles that depend on one exact dependency.
    ///
    /// # Parameters
    /// - `dependency`: Exact dependency to find.
    ///
    /// # Returns
    /// Matching immutable handles.
    ///
    /// # Errors
    /// This lookup is infallible.
    fn by_dependency(&self, dependency: &Dependency) -> Vec<DerivedResultHandle> {
        self.records
            .values()
            .filter(|record| record.key().dependencies.as_slice().contains(dependency))
            .map(DerivedResultRecord::handle)
            .collect()
    }

    /// Returns the exact dependencies recorded for a result.
    ///
    /// # Parameters
    /// - `handle`: Exact result handle.
    ///
    /// # Returns
    /// The normalized dependencies, when retained.
    fn dependencies(&self, handle: DerivedResultHandle) -> Option<&DependencySet> {
        self.records
            .get(&handle)
            .map(|record| &record.key().dependencies)
    }

    /// Finds handles associated with an exact entity revision.
    ///
    /// # Parameters
    /// - `entity`: Exact entity identity.
    /// - `revision`: Exact revision identity.
    ///
    /// # Returns
    /// Matching immutable handles.
    ///
    /// # Errors
    /// This lookup is infallible.
    fn by_entity_revision(
        &self,
        entity: EntityId,
        revision: RevisionId,
    ) -> Vec<DerivedResultHandle> {
        self.by_dependency(&Dependency::ExactRevision { entity, revision })
    }

    /// Removes a record only when its retention category is disposable.
    ///
    /// # Parameters
    /// - `self`: Repository to update.
    /// - `handle`: Exact result identity.
    ///
    /// # Returns
    /// `true` when a disposable record was removed.
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
/// # Parameters
/// - `value`: Byte repeated in the fixed-width test identifier.
///
/// # Returns
/// A test entity ID.
///
/// # Errors
/// Construction from fixed-width bytes is infallible.
pub fn entity(value: u8) -> EntityId {
    EntityId::from_bytes([value; 16])
}

/// Creates a fake immutable revision identity.
///
/// # Parameters
/// - `value`: Byte repeated in the fixed-width test identifier.
///
/// # Returns
/// A test revision ID.
///
/// # Errors
/// Construction from fixed-width bytes is infallible.
pub fn revision(value: u8) -> RevisionId {
    RevisionId::new([value; 32])
}

/// Creates a result handle from a fake immutable artifact ID.
///
/// # Parameters
/// - `value`: Byte repeated in the artifact identity.
///
/// # Returns
/// Exact derived result handle.
///
/// # Errors
/// Handle construction is infallible.
pub fn handle(value: u8) -> DerivedResultHandle {
    DerivedResultHandle {
        artifact_id: revision(value),
    }
}

/// Builds an exact profile and normalized dependency key.
///
/// # Parameters
/// - `name`: Fake derivation profile kind.
/// - `dependencies`: Exact declared dependencies.
///
/// # Returns
/// Deterministic derivation request key.
///
/// # Errors
/// Key construction is infallible for already formed identifiers.
pub fn key(name: &str, dependencies: Vec<Dependency>) -> DerivationKey {
    DerivationKey::new(
        scriberig_derive::DerivationProfile::new(name, "v1"),
        DependencySet::new(dependencies),
    )
}

/// Creates a successful immutable record for an exact key.
///
/// # Parameters
/// - `request`: Exact derivation key.
/// - `artifact`: Stable artifact identity byte.
///
/// # Returns
/// Successful derived result record.
///
/// # Errors
/// Record construction is infallible for exact keys and handles.
pub fn successful(request: DerivationKey, artifact: u8) -> DerivedResultRecord {
    DerivedResultRecord::new(request, handle(artifact), DerivedResultStatus::Succeeded)
}
