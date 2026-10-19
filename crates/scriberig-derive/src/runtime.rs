//! Provides a usable in-memory derived repository and cache resolution runtime.

use crate::dependency::{Dependency, DependencySet};
use crate::freshness::FreshnessPolicy;
use crate::repository::{DerivedRepository, RepositoryError};
use crate::result::DerivedResultRecord;
use crate::selection::{SelectionOutcome, SelectionPolicy, select};
use crate::{DerivationEvent, DerivationKey, DerivedResultHandle, EventReason};
use scriberig_core::{EntityId, RevisionId};
use std::collections::BTreeMap;

/// Stores immutable derived records in process memory.
#[derive(Clone, Debug, Default)]
pub struct InMemoryDerivedRepository {
    records: BTreeMap<DerivedResultHandle, DerivedResultRecord>,
}

impl InMemoryDerivedRepository {
    /// Creates an empty in-memory repository.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            records: BTreeMap::new(),
        }
    }
}

impl DerivedRepository for InMemoryDerivedRepository {
    /// Inserts a record without replacing an immutable handle.
    ///
    /// - `record`: Immutable result to retain.
    ///
    /// Success when inserted or already present identically.
    ///
    /// Returns a conflict when the handle already names different data.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn insert(&mut self, record: DerivedResultRecord) -> Result<(), RepositoryError> {
        if let Some(existing) = self.records.get(&record.handle()) {
            if existing != &record {
                return Err(RepositoryError::ConflictingResult {
                    handle: record.handle(),
                });
            }
            return Ok(());
        }
        self.records.insert(record.handle(), record);
        Ok(())
    }

    /// Retrieves one retained record by immutable handle.
    ///
    /// - `handle`: Immutable result identity.
    ///
    /// The retained record, when present.
    ///
    /// This lookup is infallible and returns `None` when absent.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn get(&self, handle: DerivedResultHandle) -> Option<&DerivedResultRecord> {
        self.records.get(&handle)
    }

    /// Retrieves every record with an exact derivation key.
    ///
    /// - `key`: Exact profile and dependency identity.
    ///
    /// All retained records matching the key.
    ///
    /// This lookup is infallible and returns an empty list when unmatched.
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

    /// Retrieves records that declare one exact dependency.
    ///
    /// - `dependency`: Dependency to reverse-index.
    ///
    /// Handles of records declaring the dependency.
    ///
    /// This lookup is infallible and returns an empty list when unmatched.
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

    /// Retrieves the normalized dependency set for one record.
    ///
    /// - `handle`: Immutable result identity.
    ///
    /// The dependency set when the record is retained.
    /// # Returns
    /// The operation result.
    fn dependencies(&self, handle: DerivedResultHandle) -> Option<&DependencySet> {
        self.records
            .get(&handle)
            .map(DerivedResultRecord::dependencies)
    }

    /// Retrieves records attached to one exact entity revision.
    ///
    /// - `entity`: Exact entity identity.
    /// - `revision`: Exact revision identity.
    ///
    /// Handles of records depending on the exact revision.
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
    ) -> Vec<DerivedResultHandle> {
        self.by_dependency(&Dependency::ExactRevision { entity, revision })
    }

    /// Removes a disposable record while preserving pinned and historical data.
    ///
    /// - `handle`: Immutable result identity.
    ///
    /// `true` when a disposable record was removed.
    /// # Returns
    /// The operation result.
    fn remove_disposable(&mut self, handle: DerivedResultHandle) -> bool {
        let removable = self
            .records
            .get(&handle)
            .is_some_and(|record| record.retention().0 == crate::RetentionPolicy::Disposable);
        removable && self.records.remove(&handle).is_some()
    }
}

/// Reports why a cache resolution failed.
#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeError {
    /// The repository rejected an immutable record.
    Repository(RepositoryError),
    /// The producer returned a record for a different exact request.
    KeyMismatch {
        /// Requested exact derivation key.
        requested: Box<DerivationKey>,
        /// Returned exact derivation key.
        produced: Box<DerivationKey>,
    },
}

/// Describes the result of resolving one exact derivation request.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolution {
    record: DerivedResultRecord,
    reused: bool,
}

impl Resolution {
    /// Returns the selected or newly computed record.
    #[must_use]
    pub const fn record(&self) -> &DerivedResultRecord {
        &self.record
    }

    /// Reports whether an existing record was reused.
    #[must_use]
    pub const fn reused(&self) -> bool {
        self.reused
    }
}

/// Resolves exact derivations against a repository without changing canonical state.
#[derive(Clone, Debug)]
pub struct DerivedRuntime<R> {
    repository: R,
}

impl<R> DerivedRuntime<R> {
    /// Creates a runtime over a derived-result repository.
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Borrows the underlying repository.
    #[must_use]
    pub const fn repository(&self) -> &R {
        &self.repository
    }

    /// Mutably borrows the underlying repository.
    #[must_use]
    pub const fn repository_mut(&mut self) -> &mut R {
        &mut self.repository
    }
}

impl<R: DerivedRepository> DerivedRuntime<R> {
    /// Reuses an eligible record or computes and stores one exact result.
    ///
    /// - `key`: Exact derivation request.
    /// - `policy`: Selection and freshness requirements.
    /// - `freshness`: Freshness policy for stored observations.
    /// - `compute`: Producer invoked only when no stored record qualifies.
    ///
    /// The reused or newly stored immutable result.
    ///
    /// # Returns
    /// The selected or computed immutable result.
    ///
    /// # Errors
    /// Returns an error if the producer returns a different key or storage
    /// rejects the immutable result.
    pub fn resolve<F>(
        &mut self,
        key: &DerivationKey,
        policy: &SelectionPolicy,
        freshness: &FreshnessPolicy,
        compute: F,
    ) -> Result<Resolution, RuntimeError>
    where
        F: FnOnce(&DerivationKey) -> DerivedResultRecord,
    {
        self.resolve_with_events(key, policy, freshness, compute)
            .map(|(resolution, _events)| resolution)
    }

    /// Resolves a request and returns the derived lifecycle events it produces.
    ///
    /// - `key`: Exact derivation request.
    /// - `policy`: Selection and freshness requirements.
    /// - `freshness`: Freshness policy for stored observations.
    /// - `compute`: Producer invoked only when no stored record qualifies.
    ///
    /// The resolution and derived-only lifecycle events in emission order.
    ///
    /// # Returns
    /// The resolution and lifecycle events produced by this operation.
    ///
    /// # Errors
    /// Returns an error if the producer returns a different key or storage
    /// rejects the immutable result.
    pub fn resolve_with_events<F>(
        &mut self,
        key: &DerivationKey,
        policy: &SelectionPolicy,
        freshness: &FreshnessPolicy,
        compute: F,
    ) -> Result<(Resolution, Vec<DerivationEvent>), RuntimeError>
    where
        F: FnOnce(&DerivationKey) -> DerivedResultRecord,
    {
        let mut events = vec![DerivationEvent::DerivationStarted {
            profile: key.profile.clone(),
            key: key.clone(),
            subject: None,
        }];
        let candidates: Vec<DerivedResultRecord> =
            self.repository.by_key(key).into_iter().cloned().collect();
        if let SelectionOutcome::Selected { record, .. } = select(&candidates, policy, freshness) {
            let selected = record.clone();
            events.push(DerivationEvent::DerivedResultSelected {
                profile: key.profile.clone(),
                key: key.clone(),
                result: selected.handle(),
                subject: None,
            });
            return Ok((
                Resolution {
                    record: selected,
                    reused: true,
                },
                events,
            ));
        }

        let produced = compute(key);
        if produced.key() != key {
            events.push(DerivationEvent::DerivationFailed {
                profile: key.profile.clone(),
                key: key.clone(),
                reason: EventReason::Extension("producer-key-mismatch".to_owned()),
                subject: None,
            });
            return Err(RuntimeError::KeyMismatch {
                requested: Box::new(key.clone()),
                produced: Box::new(produced.key().clone()),
            });
        }
        self.repository
            .insert(produced.clone())
            .map_err(RuntimeError::Repository)?;
        match produced.status() {
            crate::DerivedResultStatus::Succeeded => {
                events.push(DerivationEvent::DerivedResultAvailable {
                    profile: key.profile.clone(),
                    key: key.clone(),
                    result: produced.handle(),
                    subject: None,
                });
            }
            crate::DerivedResultStatus::FailedDeterministic => {
                events.push(DerivationEvent::DerivationFailed {
                    profile: key.profile.clone(),
                    key: key.clone(),
                    reason: EventReason::DeterministicFailure,
                    subject: None,
                });
            }
            crate::DerivedResultStatus::FailedTransient => {
                events.push(DerivationEvent::DerivationFailed {
                    profile: key.profile.clone(),
                    key: key.clone(),
                    reason: EventReason::TransientFailure,
                    subject: None,
                });
            }
            crate::DerivedResultStatus::Cancelled => {
                events.push(DerivationEvent::DerivationFailed {
                    profile: key.profile.clone(),
                    key: key.clone(),
                    reason: EventReason::Cancelled,
                    subject: None,
                });
            }
        }
        Ok((
            Resolution {
                record: produced,
                reused: false,
            },
            events,
        ))
    }
}
