//! Verifies the backend-independent derived-result repository contract.

use std::collections::BTreeMap;

use scriberig_core::{EntityId, RevisionId};
use scriberig_derive::{
    Dependency, DependencySet, DerivationKey, DerivationProfile, DerivedRepository,
    DerivedResultHandle, DerivedResultRecord, DerivedResultStatus, RepositoryError,
    RetentionPolicy,
};

/// Stores immutable records for contract tests.
struct MemoryRepository {
    records: BTreeMap<DerivedResultHandle, DerivedResultRecord>,
}

impl MemoryRepository {
    /// Creates an empty test repository.
    ///
    /// An empty in-memory repository.
    /// # Returns
    /// The operation result.
    fn new() -> Self {
        Self {
            records: BTreeMap::new(),
        }
    }
}

impl DerivedRepository for MemoryRepository {
    /// Inserts an immutable record or verifies an identical duplicate.
    ///
    /// - `record`: Immutable record to retain.
    ///
    /// Success when inserted or already present identically.
    ///
    /// Returns a conflict when a handle names different data.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn insert(&mut self, record: DerivedResultRecord) -> Result<(), RepositoryError> {
        let handle = record.handle();
        if let Some(existing) = self.records.get(&handle) {
            return (existing == &record)
                .then_some(())
                .ok_or(RepositoryError::ConflictingResult { handle });
        }
        self.records.insert(handle, record);
        Ok(())
    }

    /// Retrieves one exact result record.
    ///
    /// - `handle`: Exact result handle.
    ///
    /// The retained record, when present.
    ///
    /// This lookup is infallible.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn get(&self, handle: DerivedResultHandle) -> Option<&DerivedResultRecord> {
        self.records.get(&handle)
    }

    /// Retrieves every result for one exact derivation key.
    ///
    /// - `key`: Exact derivation key.
    ///
    /// All retained records for the key.
    ///
    /// This lookup is infallible.
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

    /// Retrieves handles that depend on one exact dependency.
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

    /// Retrieves the exact dependency set for one result.
    ///
    /// - `handle`: Exact result handle.
    ///
    /// The normalized dependency set, when retained.
    /// # Returns
    /// The operation result.
    fn dependencies(&self, handle: DerivedResultHandle) -> Option<&DependencySet> {
        self.records
            .get(&handle)
            .map(|record| &record.key().dependencies)
    }

    /// Retrieves results associated with one exact entity revision.
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

    /// Removes a disposable result when it is retained.
    ///
    /// - `handle`: Exact result handle.
    ///
    /// Whether a disposable record was removed.
    /// # Returns
    /// The operation result.
    fn remove_disposable(&mut self, handle: DerivedResultHandle) -> bool {
        self.records
            .get(&handle)
            .is_some_and(|record| record.retention().0 == RetentionPolicy::Disposable)
            && self.records.remove(&handle).is_some()
    }
}

/// Creates one exact result fixture.
///
/// - `handle`: Distinct test handle byte.
/// - `entity`: Exact subject entity.
/// - `revision`: Exact subject revision.
///
/// An immutable successful result record.
///
/// Construction is infallible for fixed test values.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn fixture(handle: u8, entity: EntityId, revision: RevisionId) -> DerivedResultRecord {
    let key = DerivationKey::new(
        DerivationProfile::new("test", "1"),
        DependencySet::new(vec![Dependency::ExactRevision { entity, revision }]),
    );
    DerivedResultRecord::new(
        key,
        DerivedResultHandle {
            artifact_id: RevisionId::new([handle; 32]),
        },
        DerivedResultStatus::Succeeded,
    )
}

/// Verifies exact, key, dependency, discovery, and removal lookups.
#[test]
fn repository_preserves_immutable_results_and_indexes() {
    let entity = EntityId::from_bytes([1; 16]);
    let revision = RevisionId::new([2; 32]);
    let first = fixture(3, entity, revision);
    let second = fixture(4, entity, revision);
    let key = first.key().clone();
    let handle = first.handle();
    let mut repository = MemoryRepository::new();

    repository.insert(first.clone()).expect("insert first");
    repository.insert(first).expect("identical insert is safe");
    repository
        .insert(second)
        .expect("distinct result is retained");
    assert_eq!(repository.get(handle), Some(&fixture(3, entity, revision)));
    assert_eq!(repository.by_key(&key).len(), 2);
    assert_eq!(
        repository
            .dependencies(handle)
            .expect("dependencies")
            .as_slice()
            .len(),
        1
    );
    assert_eq!(repository.by_entity_revision(entity, revision).len(), 2);
    assert!(repository.remove_disposable(handle));
    assert!(repository.get(handle).is_none());
}
