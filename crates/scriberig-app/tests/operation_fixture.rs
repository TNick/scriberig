//! Provides a deterministic operation provider for application tests.

use scriberig_app::{
    ApplicationContext, ApplicationError, ApplyRequest, BranchRequest, CommitRequest, DiffRequest,
    HistoryRequest, MergeRequest, OperationProvider, OperationSummary, RenderRequest,
};
use scriberig_core::{RootName, SnapshotId};
use std::cell::Cell;

/// Records deterministic calls for universal operation tests.
#[derive(Default)]
pub struct OperationFixture {
    calls: Cell<usize>,
}

impl OperationFixture {
    /// Creates an empty operation recorder.
    ///
    /// An operation recorder with no calls.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new() -> Self {
        Self {
            calls: Cell::new(0),
        }
    }

    /// Returns the next deterministic operation summary.
    ///
    /// - `handle`: Stable result handle.
    ///
    /// The recorded operation summary.
    /// # Returns
    /// The operation result.
    fn result(&self, handle: &str) -> OperationSummary {
        self.calls.set(self.calls.get() + 1);
        OperationSummary {
            handle: handle.to_owned(),
            snapshot: None,
            revision: None,
            branch: None,
            diff: None,
            history: None,
            rendered: None,
            validation: None,
            validation_profile: None,
            validation_diagnostics: Vec::new(),
            review_details: Vec::new(),
            conflicts: Vec::new(),
        }
    }
}

impl OperationProvider for OperationFixture {
    /// Returns an empty structured merge result for dispatch tests.
    ///
    /// - `_context`: Application context.
    /// - `request`: Exact merge input snapshots.
    ///
    /// A conflict-free merge result retaining the supplied identities.
    ///
    /// This fixture never fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn merge(
        &self,
        _context: &ApplicationContext,
        request: &MergeRequest,
    ) -> Result<sr_change::MergeResult, ApplicationError> {
        Ok(sr_change::MergeResult {
            base: request.base,
            left: request.left,
            right: request.right,
            resolved_deltas: Vec::new(),
            conflicts: Vec::new(),
            result: None,
        })
    }

    /// Resolves every fixture root to one exact snapshot.
    ///
    /// - `_root`: Fixture root name.
    ///
    /// The deterministic snapshot.
    ///
    /// This fixture never fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn resolve_root(&self, _root: &RootName) -> Result<SnapshotId, ApplicationError> {
        Ok(SnapshotId::new([1; 32]))
    }

    /// Records a diff request.
    ///
    /// - `_context`: Application context.
    /// - `_request`: Diff request.
    ///
    /// A deterministic operation summary.
    ///
    /// This fixture never fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn diff(
        &self,
        _context: &ApplicationContext,
        _request: &DiffRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Ok(self.result("diff"))
    }

    /// Records a history request.
    ///
    /// - `_context`: Application context.
    /// - `_request`: History request.
    ///
    /// A deterministic operation summary.
    ///
    /// This fixture never fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn history(
        &self,
        _context: &ApplicationContext,
        _request: &HistoryRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Ok(self.result("history"))
    }

    /// Records a render request.
    ///
    /// - `_context`: Application context.
    /// - `_request`: Render request.
    ///
    /// A deterministic operation summary.
    ///
    /// This fixture never fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn render(
        &self,
        _context: &ApplicationContext,
        _request: &RenderRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Ok(self.result("render"))
    }

    /// Records a branch request.
    ///
    /// - `_context`: Application context.
    /// - `_request`: Branch request.
    ///
    /// A deterministic operation summary.
    ///
    /// This fixture never fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn branch(
        &self,
        _context: &ApplicationContext,
        _request: &BranchRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Ok(self.result("branch"))
    }

    /// Records an apply request.
    ///
    /// - `_context`: Application context.
    /// - `_request`: Apply request.
    ///
    /// A deterministic operation summary.
    ///
    /// This fixture never fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn apply(
        &self,
        _context: &ApplicationContext,
        _request: &ApplyRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Ok(self.result("apply"))
    }

    /// Records a commit request.
    ///
    /// - `_context`: Application context.
    /// - `_request`: Commit request.
    ///
    /// A deterministic operation summary.
    ///
    /// This fixture never fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn commit(
        &self,
        _context: &ApplicationContext,
        _request: &CommitRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Ok(self.result("commit"))
    }
}
