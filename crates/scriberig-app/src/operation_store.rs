//! Provides storage-backed semantic diff execution.

use crate::{
    ApplicationContext, ApplicationError, ApplyRequest, BranchRequest, CommitRequest, DiffRequest,
    HistoryRequest, MergeRequest, MigrationDelta, MigrationDetail, OperationProvider,
    OperationSummary, RenderRequest,
};
use scriberig_store::{
    RootError, RootStore, SnapshotChange, SnapshotDiff, SnapshotRelationChange, SnapshotStore,
};
use sr_change::{EntityDelta, HistoryGraph, MigrationReference};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Mutex;

#[path = "operation_store_helpers.rs"]
mod operation_store_helpers;

use operation_store_helpers::{map_root_error, unsupported};
/// Executes exact snapshot diffs through the canonical snapshot store.
pub struct StoreOperationProvider<S> {
    store: S,
    history: Option<HistoryGraph>,
    migrations: Mutex<BTreeMap<String, sr_change::Migration>>,
}

impl<S> StoreOperationProvider<S> {
    /// Creates a storage-backed operation provider.
    ///
    /// - `store`: Snapshot backend owning exact immutable states.
    ///
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
    /// - `self`: Provider being configured.
    /// - `history`: Canonical migration topology.
    ///
    /// Provider with history access enabled.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_history(mut self, history: HistoryGraph) -> Self {
        self.history = Some(history);
        self
    }

    /// Returns the underlying snapshot store.
    ///
    /// - `self`: Provider to inspect.
    ///
    /// The canonical snapshot backend.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }
}

impl<S: SnapshotStore + RootStore> OperationProvider for StoreOperationProvider<S> {
    /// Merges exact stored candidates through the existing semantic merge owner.
    ///
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `request`: Exact base and candidate snapshots.
    ///
    /// Structured merge result with conflicts or a retained candidate snapshot.
    ///
    /// Returns snapshot-unavailable when an input cannot be read or retained.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn merge(
        &self,
        _context: &ApplicationContext,
        request: &MergeRequest,
    ) -> Result<sr_change::MergeResult, ApplicationError> {
        sr_change::merge_snapshots_into_store(
            &self.store,
            sr_change::MergeInput {
                base: request.base,
                left: request.left,
                right: request.right,
            },
            None,
        )
        .map_err(|_| ApplicationError::SnapshotUnavailable {
            snapshot: request.base,
        })
    }

    /// Reads one registered migration without changing canonical state.
    ///
    /// - `self`: Storage-backed operation provider.
    /// - `id`: Stable migration handle.
    ///
    /// Snapshot-bound migration detail.
    ///
    /// Returns not-found when the migration handle is unknown.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn migration_detail(&self, id: &str) -> Result<MigrationDetail, ApplicationError> {
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
        Ok(MigrationDetail {
            id: id.to_owned(),
            base: migration.base,
            result: migration.result,
            deltas: migration
                .deltas
                .iter()
                .map(|delta| MigrationDelta {
                    entity: delta.entity,
                    before: delta.before,
                    after: delta.after,
                })
                .collect(),
        })
    }

    /// Computes an exact snapshot diff through the store contract.
    ///
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `request`: Exact snapshots to compare.
    ///
    /// Summary containing the stable diff handle and right snapshot.
    ///
    /// Returns an internal application error when either snapshot is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
            validation: None,
            validation_profile: None,
            validation_diagnostics: Vec::new(),
            review_details: Vec::new(),
            conflicts: Vec::new(),
        })
    }

    /// Reports that history storage is not configured by this provider.
    ///
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `request`: History request.
    ///
    /// Never returns a history summary.
    ///
    /// Returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn history(
        &self,
        _context: &ApplicationContext,
        request: &HistoryRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        let snapshot = match request.snapshot {
            crate::SnapshotSelector::Exact(snapshot) => snapshot,
            crate::SnapshotSelector::NamedRoot(_) => {
                return Err(unsupported("history named-root resolution"));
            }
        };
        let history = match &self.history {
            Some(history) => history.clone(),
            None => self.history_for_snapshot(snapshot)?,
        };
        Ok(OperationSummary {
            handle: format!("history:{snapshot}:{}", history.migrations().len()),
            snapshot: Some(snapshot),
            revision: None,
            branch: None,
            diff: None,
            history: Some(history.migrations().into_iter().cloned().collect()),
            rendered: None,
            validation: None,
            validation_profile: None,
            validation_diagnostics: Vec::new(),
            review_details: Vec::new(),
            conflicts: Vec::new(),
        })
    }

    /// Reports that projection rendering is not owned by this provider.
    ///
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `_request`: Render request.
    ///
    /// Never returns a render summary.
    ///
    /// Returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn render(
        &self,
        _context: &ApplicationContext,
        _request: &RenderRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Err(unsupported("render"))
    }

    /// Records a branch request at its exact immutable base.
    ///
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `request`: Branch request.
    ///
    /// Branch handle summary with its exact base snapshot.
    ///
    /// Returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
            validation: None,
            validation_profile: None,
            validation_diagnostics: Vec::new(),
            review_details: Vec::new(),
            conflicts: Vec::new(),
        })
    }

    /// Applies a registered migration without promoting a named root.
    ///
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `request`: Apply request.
    ///
    /// Candidate summary containing the resulting immutable snapshot.
    ///
    /// Returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn apply(
        &self,
        _context: &ApplicationContext,
        request: &ApplyRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        let (changes, relation_changes) = self.semantic_changes_for(&request.change)?;
        let candidate = self
            .store
            .apply_changes_with_relations(request.base_snapshot, &changes, &relation_changes)
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
            validation: None,
            validation_profile: None,
            validation_diagnostics: Vec::new(),
            review_details: Vec::new(),
            conflicts: Vec::new(),
        })
    }

    /// Applies a registered migration and promotes the conventional `main` root.
    ///
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `request`: Commit request.
    ///
    /// Promotion summary containing the resulting immutable snapshot.
    ///
    /// Returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn commit(
        &self,
        _context: &ApplicationContext,
        request: &CommitRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        let (changes, relation_changes) = self.semantic_changes_for(&request.change)?;
        let candidate = self
            .store
            .apply_changes_with_relations(request.base_snapshot, &changes, &relation_changes)
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
            validation: None,
            validation_profile: None,
            validation_diagnostics: Vec::new(),
            review_details: Vec::new(),
            conflicts: Vec::new(),
        })
    }
}
