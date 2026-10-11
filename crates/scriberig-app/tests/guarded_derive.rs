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

const PUBLIC: EntityId = EntityId::new([1; 16]);
const RESTRICTED: EntityId = EntityId::new([2; 16]);

/// Stores immutable test artifacts for the repository adapter.
struct MemoryRepository {
    records: BTreeMap<DerivedResultHandle, DerivedResultRecord>,
}

impl MemoryRepository {
    /// Creates an empty derived repository.
    ///
    /// # Returns
    /// An empty in-memory repository.
    fn new() -> Self {
        Self {
            records: BTreeMap::new(),
        }
    }
}

impl DerivedRepository for MemoryRepository {
    /// Inserts an immutable test record.
    ///
    /// # Parameters
    /// - `record`: Record to retain.
    ///
    /// # Returns
    /// Success when the handle is new or identical.
    ///
    /// # Errors
    /// Returns a conflict when the handle has different contents.
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
    /// # Parameters
    /// - `handle`: Record handle to find.
    ///
    /// # Returns
    /// The retained record, when present.
    ///
    /// # Errors
    /// This lookup is infallible.
    fn get(&self, handle: DerivedResultHandle) -> Option<&DerivedResultRecord> {
        self.records.get(&handle)
    }

    /// Retrieves records matching one exact key.
    ///
    /// # Parameters
    /// - `key`: Key to match.
    ///
    /// # Returns
    /// Matching records.
    ///
    /// # Errors
    /// This lookup is infallible.
    fn by_key(&self, key: &DerivationKey) -> Vec<&DerivedResultRecord> {
        self.records
            .values()
            .filter(|record| record.key() == key)
            .collect()
    }

    /// Retrieves handles declaring one dependency.
    ///
    /// # Parameters
    /// - `dependency`: Dependency to match.
    ///
    /// # Returns
    /// Matching artifact handles.
    ///
    /// # Errors
    /// This lookup is infallible.
    fn by_dependency(&self, dependency: &Dependency) -> Vec<DerivedResultHandle> {
        self.records
            .values()
            .filter(|record| record.dependencies().as_slice().contains(dependency))
            .map(DerivedResultRecord::handle)
            .collect()
    }

    /// Retrieves the exact dependency set for one handle.
    ///
    /// # Parameters
    /// - `handle`: Artifact handle to inspect.
    ///
    /// # Returns
    /// The dependency set, when retained.
    fn dependencies(&self, handle: DerivedResultHandle) -> Option<&DependencySet> {
        self.records
            .get(&handle)
            .map(DerivedResultRecord::dependencies)
    }

    /// Retrieves records for one exact entity revision.
    ///
    /// # Parameters
    /// - `entity`: Entity dependency to match.
    /// - `revision`: Revision dependency to match.
    ///
    /// # Returns
    /// Matching artifact handles.
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

    /// Removes one disposable test record.
    ///
    /// # Parameters
    /// - `handle`: Artifact handle to remove.
    ///
    /// # Returns
    /// Whether a record was removed.
    fn remove_disposable(&mut self, handle: DerivedResultHandle) -> bool {
        self.records.remove(&handle).is_some()
    }
}

/// Builds one capability with no additional constraints.
///
/// # Parameters
/// - `scope`: Scope covered by the grant.
///
/// # Returns
/// A query capability for the supplied scope.
fn read_capability(scope: CapabilityScope) -> Capability {
    Capability {
        scope,
        actions: BTreeSet::from([CapabilityAction::Query]),
        constraints: CapabilityConstraints::unrestricted(),
    }
}

/// Builds an application context from complete scoped grants.
///
/// # Parameters
/// - `grants`: Grants to evaluate.
///
/// # Returns
/// A context containing the grants.
fn context(grants: impl IntoIterator<Item = Capability>) -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext::with_grants(grants),
        ..ApplicationContext::default()
    }
}

/// Builds a successful immutable artifact with exact dependencies.
///
/// # Parameters
/// - `handle`: Artifact identity byte.
/// - `dependencies`: Exact semantic dependencies.
///
/// # Returns
/// An immutable successful artifact record.
///
/// # Errors
/// Construction is infallible for fixed test values.
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
