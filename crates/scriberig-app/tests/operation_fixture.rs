//! Provides a deterministic operation provider for application tests.

use scriberig_app::{
    ApplicationContext, ApplicationError, ApplyRequest, BranchRequest, CommitRequest, DiffRequest,
    HistoryRequest, OperationProvider, OperationSummary, RenderRequest,
};
use scriberig_core::{RootName, SnapshotId};
use std::cell::Cell;

/// Records deterministic calls for universal operation tests.
pub struct OperationFixture {
    calls: Cell<usize>,
}

impl OperationFixture {
    /// Creates an empty operation recorder.
    ///
    /// # Returns
    /// An operation recorder with no calls.
    #[must_use]
    pub fn new() -> Self {
        Self {
            calls: Cell::new(0),
        }
    }

    /// Returns the next deterministic operation summary.
    ///
    /// # Parameters
    /// - `handle`: Stable result handle.
    ///
    /// # Returns
    /// The recorded operation summary.
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
        }
    }
}

impl OperationProvider for OperationFixture {
    /// Resolves every fixture root to one exact snapshot.
    ///
    /// # Parameters
    /// - `_root`: Fixture root name.
    ///
    /// # Returns
    /// The deterministic snapshot.
    ///
    /// # Errors
    /// This fixture never fails.
    fn resolve_root(&self, _root: &RootName) -> Result<SnapshotId, ApplicationError> {
        Ok(SnapshotId::new([1; 32]))
    }

    /// Records a diff request.
    ///
    /// # Parameters
    /// - `_context`: Application context.
    /// - `_request`: Diff request.
    ///
    /// # Returns
    /// A deterministic operation summary.
    ///
    /// # Errors
    /// This fixture never fails.
    fn diff(
        &self,
        _context: &ApplicationContext,
        _request: &DiffRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Ok(self.result("diff"))
    }

    /// Records a history request.
    ///
    /// # Parameters
    /// - `_context`: Application context.
    /// - `_request`: History request.
    ///
    /// # Returns
    /// A deterministic operation summary.
    ///
    /// # Errors
    /// This fixture never fails.
    fn history(
        &self,
        _context: &ApplicationContext,
        _request: &HistoryRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Ok(self.result("history"))
    }

    /// Records a render request.
    ///
    /// # Parameters
    /// - `_context`: Application context.
    /// - `_request`: Render request.
    ///
    /// # Returns
    /// A deterministic operation summary.
    ///
    /// # Errors
    /// This fixture never fails.
    fn render(
        &self,
        _context: &ApplicationContext,
        _request: &RenderRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Ok(self.result("render"))
    }

    /// Records a branch request.
    ///
    /// # Parameters
    /// - `_context`: Application context.
    /// - `_request`: Branch request.
    ///
    /// # Returns
    /// A deterministic operation summary.
    ///
    /// # Errors
    /// This fixture never fails.
    fn branch(
        &self,
        _context: &ApplicationContext,
        _request: &BranchRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Ok(self.result("branch"))
    }

    /// Records an apply request.
    ///
    /// # Parameters
    /// - `_context`: Application context.
    /// - `_request`: Apply request.
    ///
    /// # Returns
    /// A deterministic operation summary.
    ///
    /// # Errors
    /// This fixture never fails.
    fn apply(
        &self,
        _context: &ApplicationContext,
        _request: &ApplyRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Ok(self.result("apply"))
    }

    /// Records a commit request.
    ///
    /// # Parameters
    /// - `_context`: Application context.
    /// - `_request`: Commit request.
    ///
    /// # Returns
    /// A deterministic operation summary.
    ///
    /// # Errors
    /// This fixture never fails.
    fn commit(
        &self,
        _context: &ApplicationContext,
        _request: &CommitRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Ok(self.result("commit"))
    }
}
