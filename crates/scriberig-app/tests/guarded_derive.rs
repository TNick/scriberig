//! Verifies dependency-aware authorization for shared derived cache entries.

use scriberig_app::{
    ApplicationContext, Capability, CapabilityAction, CapabilityConstraints, CapabilityContext,
    CapabilityScope, GuardedDerivedRepository,
};
use scriberig_core::{EntityId, RevisionId};
use scriberig_derive::{
    Dependency, DependencySet, DerivationKey, DerivationProfile, DerivedRepository,
    DerivedResultHandle, DerivedResultRecord, DerivedResultStatus, RepositoryError,
};
use std::collections::{BTreeMap, BTreeSet};

const PUBLIC: EntityId = EntityId::from_bytes([1; 16]);
const RESTRICTED: EntityId = EntityId::from_bytes([2; 16]);

/// Stores immutable test artifacts for the repository adapter.
struct MemoryRepository {
    records: BTreeMap<DerivedResultHandle, DerivedResultRecord>,
}

impl MemoryRepository {
    /// Creates an empty derived repository.
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
    /// Inserts an immutable test record.
    ///
    /// - `record`: Record to retain.
    ///
    /// Success when the handle is new or identical.
    ///
    /// Returns a conflict when the handle has different contents.
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

    /// Retrieves one immutable test record.
    ///
    /// - `handle`: Record handle to find.
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

    /// Retrieves records matching one exact key.
    ///
    /// - `key`: Key to match.
    ///
    /// Matching records.
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

    /// Retrieves handles declaring one dependency.
    ///
    /// - `dependency`: Dependency to match.
    ///
    /// Matching artifact handles.
    ///
    /// This lookup is infallible.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn by_dependency(&self, dependency: &Dependency) -> Vec<DerivedResultHandle> {
        self.records
            .values()
            .filter(|record| record.dependencies().as_slice().contains(dependency))
            .map(DerivedResultRecord::handle)
            .collect()
    }

    /// Retrieves the exact dependency set for one handle.
    ///
    /// - `handle`: Artifact handle to inspect.
    ///
    /// The dependency set, when retained.
    /// # Returns
    /// The operation result.
    fn dependencies(&self, handle: DerivedResultHandle) -> Option<&DependencySet> {
        self.records
            .get(&handle)
            .map(DerivedResultRecord::dependencies)
    }

    /// Retrieves records for one exact entity revision.
    ///
    /// - `entity`: Entity dependency to match.
    /// - `revision`: Revision dependency to match.
    ///
    /// Matching artifact handles.
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

    /// Removes one disposable test record.
    ///
    /// - `handle`: Artifact handle to remove.
    ///
    /// Whether a record was removed.
    /// # Returns
    /// The operation result.
    fn remove_disposable(&mut self, handle: DerivedResultHandle) -> bool {
        self.records.remove(&handle).is_some()
    }
}

/// Builds one capability with no additional constraints.
///
/// - `scope`: Scope covered by the grant.
///
/// A query capability for the supplied scope.
/// # Returns
/// The operation result.
fn read_capability(scope: CapabilityScope) -> Capability {
    Capability {
        scope,
        actions: BTreeSet::from([CapabilityAction::Query]),
        constraints: CapabilityConstraints::unrestricted(),
    }
}

/// Builds an application context from complete scoped grants.
///
/// - `grants`: Grants to evaluate.
///
/// A context containing the grants.
/// # Returns
/// The operation result.
fn context(grants: impl IntoIterator<Item = Capability>) -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext::with_grants(grants),
        ..ApplicationContext::default()
    }
}

/// Builds a successful immutable artifact with exact dependencies.
///
/// - `handle`: Artifact identity byte.
/// - `dependencies`: Exact semantic dependencies.
///
/// An immutable successful artifact record.
///
/// Construction is infallible for fixed test values.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn record(handle: u8, dependencies: Vec<Dependency>) -> DerivedResultRecord {
    DerivedResultRecord::new(
        DerivationKey::new(
            DerivationProfile::new("summary", "v1"),
            DependencySet::new(dependencies),
        ),
        DerivedResultHandle {
            artifact_id: RevisionId::new([handle; 32]),
        },
        DerivedResultStatus::Succeeded,
    )
}

/// Denies a restricted cache hit while retaining the shared artifact.
#[test]
fn cache_hit_requires_all_dependency_access() {
    let base = record(
        4,
        vec![Dependency::ExactRevision {
            entity: RESTRICTED,
            revision: RevisionId::new([5; 32]),
        }],
    );
    let handle = base.handle();
    let mut repository = MemoryRepository::new();
    repository.insert(base).expect("insert artifact");
    let guarded = GuardedDerivedRepository::new(repository);

    let denied = context([read_capability(CapabilityScope::Entity(PUBLIC))]);
    assert!(guarded.get(&denied, handle).is_none());
    assert!(guarded.inner().get(handle).is_some());

    let allowed = context([read_capability(CapabilityScope::Project)]);
    assert!(guarded.get(&allowed, handle).is_some());
}

/// Recursively checks dependencies that are themselves derived artifacts.
#[test]
fn nested_artifacts_preserve_restrictions() {
    let base = record(
        6,
        vec![Dependency::ExactRevision {
            entity: RESTRICTED,
            revision: RevisionId::new([7; 32]),
        }],
    );
    let child = record(
        8,
        vec![Dependency::DerivedResult(base.handle().artifact_id)],
    );
    let child_handle = child.handle();
    let mut repository = MemoryRepository::new();
    repository.insert(base).expect("insert base");
    repository.insert(child).expect("insert child");
    let guarded = GuardedDerivedRepository::new(repository);

    let denied = context([read_capability(CapabilityScope::Entity(PUBLIC))]);
    assert!(guarded.get(&denied, child_handle).is_none());
    let allowed = context([read_capability(CapabilityScope::Project)]);
    assert!(guarded.get(&allowed, child_handle).is_some());
}
