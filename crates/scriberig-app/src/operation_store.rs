//! Provides storage-backed semantic diff execution.

use crate::{
    ApplicationContext, ApplicationError, ApplyRequest, BranchRequest, CommitRequest, DiffRequest,
    HistoryRequest, OperationProvider, OperationSummary, RenderRequest,
};
use scriberig_store::{RootError, RootStore, SnapshotChange, SnapshotStore};
use sr_change::HistoryGraph;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// Executes exact snapshot diffs through the canonical snapshot store.
pub struct StoreOperationProvider<S> {
    store: S,
    history: Option<HistoryGraph>,
    migrations: Mutex<BTreeMap<String, sr_change::Migration>>,
}

impl<S> StoreOperationProvider<S> {
    /// Creates a storage-backed operation provider.
    ///
    /// # Parameters
    /// - `store`: Snapshot backend owning exact immutable states.
    ///
    /// # Returns
    /// A provider for storage-owned operations.
    #[must_use]
    pub const fn new(store: S) -> Self {
        Self {
            store,
            history: None,
            migrations: Mutex::new(BTreeMap::new()),
        }
    }

    /// Installs an immutable history graph for history summaries.
    ///
    /// # Parameters
    /// - `self`: Provider being configured.
    /// - `history`: Canonical migration topology.
    ///
    /// # Returns
    /// Provider with history access enabled.
    #[must_use]
    pub fn with_history(mut self, history: HistoryGraph) -> Self {
        self.history = Some(history);
        self
    }

    /// Returns the underlying snapshot store.
    ///
    /// # Parameters
    /// - `self`: Provider to inspect.
    ///
    /// # Returns
    /// The canonical snapshot backend.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }

    /// Registers a canonical migration for later apply or commit execution.
    ///
    /// # Parameters
    /// - `self`: Provider owning the migration registry.
    /// - `id`: Stable migration handle supplied by the caller.
    /// - `migration`: Validated canonical migration.
    ///
    /// # Returns
    /// `true` when the handle was new, or `false` when it already existed.
    pub fn register_migration(
        &self,
        id: impl Into<String>,
        migration: sr_change::Migration,
    ) -> bool {
        let Ok(mut migrations) = self.migrations.lock() else {
            return false;
        };
        migrations.insert(id.into(), migration).is_none()
    }

    /// Resolves a registered migration into storage changes.
    ///
    /// # Parameters
    /// - `self`: Provider owning canonical migrations.
    /// - `id`: Stable migration handle.
    ///
    /// # Returns
    /// Exact storage changes in canonical entity order.
    ///
    /// # Errors
    /// Returns not-found when the migration handle is unknown.
    fn changes_for(&self, id: &str) -> Result<Vec<SnapshotChange>, ApplicationError> {
        let migrations = self
            .migrations
            .lock()
            .map_err(|_| ApplicationError::Internal {
                message: "migration registry lock failed".to_owned(),
            })?;
        let migration = migrations
            .get(id)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: id.to_owned(),
            })?;
        Ok(migration
            .deltas
            .iter()
            .map(|delta| match delta.after {
                Some(revision) => SnapshotChange::Set(delta.entity, revision),
                None => SnapshotChange::Remove(delta.entity),
            })
            .collect())
    }
}

impl<S: SnapshotStore + RootStore> OperationProvider for StoreOperationProvider<S> {
    /// Computes an exact snapshot diff through the store contract.
    ///
    /// # Parameters
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `request`: Exact snapshots to compare.
    ///
    /// # Returns
    /// Summary containing the stable diff handle and right snapshot.
    ///
    /// # Errors
    /// Returns an internal application error when either snapshot is unavailable.
    fn diff(
        &self,
        _context: &ApplicationContext,
        request: &DiffRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        let diff = sr_change::compare(&self.store, request.from, request.to).map_err(|error| {
            ApplicationError::SnapshotUnavailable {
                snapshot: match error {
                    scriberig_store::SnapshotError::UnknownSnapshot(snapshot) => snapshot,
                    scriberig_store::SnapshotError::InvalidEncoding(_)
                    | scriberig_store::SnapshotError::Backend(_) => request.to,
                },
            }
        })?;
        Ok(OperationSummary {
            handle: format!("diff:{}:{}:{}", request.from, request.to, diff.deltas.len()),
            snapshot: Some(request.to),
            revision: None,
            branch: None,
            diff: Some(diff),
            history: None,
            rendered: None,
        })
    }

    /// Reports that history storage is not configured by this provider.
    ///
    /// # Parameters
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `request`: History request.
    ///
    /// # Returns
    /// Never returns a history summary.
    ///
    /// # Errors
    /// Returns an unsupported-operation error.
    fn history(
        &self,
        _context: &ApplicationContext,
        request: &HistoryRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        let Some(history) = &self.history else {
            return Err(unsupported("history"));
        };
        let snapshot = match request.snapshot {
            crate::SnapshotSelector::Exact(snapshot) => snapshot,
            crate::SnapshotSelector::NamedRoot(_) => {
                return Err(unsupported("history named-root resolution"));
            }
        };
        Ok(OperationSummary {
            handle: format!("history:{snapshot}:{}", history.migrations().len()),
            snapshot: Some(snapshot),
            revision: None,
            branch: None,
            diff: None,
            history: Some(history.migrations().into_iter().cloned().collect()),
            rendered: None,
        })
    }

    /// Reports that projection rendering is not owned by this provider.
    ///
    /// # Parameters
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `_request`: Render request.
    ///
    /// # Returns
    /// Never returns a render summary.
    ///
    /// # Errors
    /// Returns an unsupported-operation error.
    fn render(
        &self,
        _context: &ApplicationContext,
        _request: &RenderRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Err(unsupported("render"))
    }

    /// Records a branch request at its exact immutable base.
    ///
    /// # Parameters
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `request`: Branch request.
    ///
    /// # Returns
    /// Branch handle summary with its exact base snapshot.
    ///
    /// # Errors
    /// Returns an unsupported-operation error.
    fn branch(
        &self,
        _context: &ApplicationContext,
        request: &BranchRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        let snapshot = match request.base {
            crate::SnapshotSelector::Exact(snapshot) => snapshot,
            crate::SnapshotSelector::NamedRoot(_) => {
                return Err(unsupported("branch named-root resolution"));
            }
        };
        self.store
            .set_root(request.name.clone(), snapshot)
            .map_err(|error| map_root_error(&error))?;
        Ok(OperationSummary {
            handle: format!("branch:{}:{}", request.name, snapshot),
            snapshot: Some(snapshot),
            revision: None,
            branch: Some(request.name.clone()),
            diff: None,
            history: None,
            rendered: None,
        })
    }

    /// Applies a registered migration without promoting a named root.
    ///
    /// # Parameters
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `request`: Apply request.
    ///
    /// # Returns
    /// Candidate summary containing the resulting immutable snapshot.
    ///
    /// # Errors
    /// Returns an unsupported-operation error.
    fn apply(
        &self,
        _context: &ApplicationContext,
        request: &ApplyRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        let changes = self.changes_for(&request.change)?;
        let candidate = self
            .store
            .apply_changes(request.base_snapshot, &changes)
            .map_err(|_| ApplicationError::SnapshotUnavailable {
                snapshot: request.base_snapshot,
            })?;
        Ok(OperationSummary {
            handle: request.change.clone(),
            snapshot: Some(candidate),
            revision: None,
            branch: request.branch.clone(),
            diff: None,
            history: None,
            rendered: None,
        })
    }

    /// Applies a registered migration and promotes the conventional `main` root.
    ///
    /// # Parameters
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `request`: Commit request.
    ///
    /// # Returns
    /// Promotion summary containing the resulting immutable snapshot.
    ///
    /// # Errors
    /// Returns an unsupported-operation error.
    fn commit(
        &self,
        _context: &ApplicationContext,
        request: &CommitRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        let changes = self.changes_for(&request.change)?;
        let candidate = self
            .store
            .apply_changes(request.base_snapshot, &changes)
            .map_err(|_| ApplicationError::SnapshotUnavailable {
                snapshot: request.base_snapshot,
            })?;
        let root = request
            .branch
            .clone()
            .unwrap_or_else(|| scriberig_core::RootName::new("main").expect("main root"));
        self.store
            .compare_and_set_root(root.clone(), Some(request.base_snapshot), candidate)
            .map_err(|error| map_root_error(&error))?;
        Ok(OperationSummary {
            handle: request.change.clone(),
            snapshot: Some(candidate),
            revision: None,
            branch: Some(root),
            diff: None,
            history: None,
            rendered: None,
        })
    }
}

/// Creates a stable unsupported-operation error.
///
/// # Parameters
/// - `operation`: Operation name that lacks a configured provider.
///
/// # Returns
/// Structured unsupported-operation failure.
fn unsupported(operation: &str) -> ApplicationError {
    ApplicationError::UnsupportedOperation {
        operation: operation.to_owned(),
    }
}

/// Maps optimistic root failures without leaking backend details.
///
/// # Parameters
/// - `error`: Root-store failure to translate.
///
/// # Returns
/// Structured application conflict or internal failure.
fn map_root_error(error: &RootError) -> ApplicationError {
    match error {
        RootError::Conflict {
            expected, actual, ..
        } => ApplicationError::Conflict {
            subject: "main".to_owned(),
            details: Box::new(crate::ConflictDetails {
                left: expected.map(|value| value.to_string()),
                right: actual.map(|value| value.to_string()),
                ..crate::ConflictDetails::default()
            }),
        },
        RootError::Backend(_) => ApplicationError::Internal {
            message: "root promotion failed".to_owned(),
        },
    }
}
