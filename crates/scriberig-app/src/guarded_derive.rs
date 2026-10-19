//! Enforces application capability checks before exposing derived artifacts.

use crate::{ApplicationContext, CapabilityAction, CapabilityScope, OperationKind};
use scriberig_core::RevisionId;
use scriberig_derive::{Dependency, DerivedRepository, DerivedResultHandle, DerivedResultRecord};
use std::collections::BTreeSet;

/// Wraps a shared derived repository with dependency-aware application checks.
pub struct GuardedDerivedRepository<R> {
    /// Immutable cache repository shared by all principals.
    inner: R,
}

impl<R> GuardedDerivedRepository<R> {
    /// Creates a guarded view over a reusable derived repository.
    ///
    /// - `inner`: Shared policy-free derived repository.
    ///
    /// A repository adapter that checks dependencies on reads.
    #[must_use]
    pub const fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Borrows the shared repository without applying application policy.
    ///
    /// The underlying immutable cache repository.
    #[must_use]
    pub const fn inner(&self) -> &R {
        &self.inner
    }
}

impl<R: DerivedRepository> GuardedDerivedRepository<R> {
    /// Returns a cached artifact only when every dependency is readable.
    ///
    /// - `context`: Principal capabilities for the cache request.
    /// - `handle`: Exact immutable artifact handle.
    ///
    /// The cached artifact when retained and authorized; otherwise `None`.
    ///
    /// This lookup is infallible; inaccessible artifacts are represented by
    /// `None`.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn get(
        &self,
        context: &ApplicationContext,
        handle: DerivedResultHandle,
    ) -> Option<&DerivedResultRecord> {
        let mut visiting = BTreeSet::new();
        self.authorized_record(context, handle, &mut visiting)
    }

    /// Checks one artifact and recursively checks nested derived dependencies.
    fn authorized_record<'a>(
        &'a self,
        context: &ApplicationContext,
        handle: DerivedResultHandle,
        visiting: &mut BTreeSet<RevisionId>,
    ) -> Option<&'a DerivedResultRecord> {
        if !visiting.insert(handle.artifact_id) {
            return None;
        }
        let record = self.inner.get(handle)?;
        let authorized = record
            .dependencies()
            .as_slice()
            .iter()
            .all(|dependency| self.dependency_visible(context, dependency, visiting));
        visiting.remove(&handle.artifact_id);
        authorized.then_some(record)
    }

    /// Checks one declared dependency without inspecting its payload.
    ///
    /// - `context`: Principal capabilities for the cache request.
    /// - `dependency`: Declared dependency to authorize.
    /// - `visiting`: Handles currently being checked for cycle detection.
    ///
    /// `true` when the dependency and all nested dependencies are readable.
    /// # Returns
    /// The operation result.
    fn dependency_visible(
        &self,
        context: &ApplicationContext,
        dependency: &Dependency,
        visiting: &mut BTreeSet<RevisionId>,
    ) -> bool {
        match dependency {
            Dependency::ExactRevision { entity, .. } => {
                can_read(context, &CapabilityScope::Entity(*entity), None)
            }
            Dependency::Snapshot(snapshot) => {
                can_read(context, &CapabilityScope::Project, Some(*snapshot))
            }
            Dependency::Scope(_) | Dependency::Configuration(_) => {
                can_read(context, &CapabilityScope::Project, None)
            }
            Dependency::DerivedResult(artifact_id) => self
                .authorized_record(
                    context,
                    DerivedResultHandle {
                        artifact_id: *artifact_id,
                    },
                    visiting,
                )
                .is_some(),
        }
    }
}

/// Checks query or inspect authority for one dependency scope.
///
/// - `context`: Principal capabilities for the cache request.
/// - `scope`: Dependency scope to authorize.
/// - `snapshot`: Exact snapshot constraint, when available.
///
/// `true` when query or inspect authority covers the dependency.
/// # Returns
/// The operation result.
fn can_read(
    context: &ApplicationContext,
    scope: &CapabilityScope,
    snapshot: Option<scriberig_core::SnapshotId>,
) -> bool {
    context.capabilities.permits(
        &CapabilityAction::Query,
        scope,
        snapshot,
        Some(OperationKind::Query),
    ) || context.capabilities.permits(
        &CapabilityAction::Inspect,
        scope,
        snapshot,
        Some(OperationKind::Inspect),
    )
}
