//! Verifies universal application dispatch across read and operation providers.

use std::cell::Cell;

use scriberig_app::{
    ApplicationContext, ApplicationRequest, ApplicationResult, ApplyRequest, BranchRequest,
    CommitRequest, CompositeBackend, DiffRequest, HistoryRequest, OperationProvider,
    OperationSummary, RenderRequest, SnapshotSelector,
};
use scriberig_core::{RootName, SnapshotId};
use scriberig_query::{
    FindRequest, FindResult, FindService, FollowRequest, FollowResult, InspectRequest,
    InspectService, Inspection, ListRequest, ListResult, QueryError, QueryResult, QuerySpec,
    StructuredQueryService, TraverseService,
};

/// Provides explicit unsupported errors for read-dispatch assertions.
struct ReadFixture;

impl InspectService for ReadFixture {
    /// Returns an explicit provider error.
    ///
    /// # Parameters
    /// - `self`: Fixture provider.
    /// - `_request`: Inspection request.
    ///
    /// # Returns
    /// Never returns an inspection.
    ///
    /// # Errors
    /// Always returns the fixture capability error.
    fn inspect(&self, _request: &InspectRequest) -> Result<Inspection, QueryError> {
        Err(QueryError::UnsupportedCapability {
            operation: scriberig_query::ReadOperation::Inspect,
            capability: "fixture".to_owned(),
        })
    }
}

impl TraverseService for ReadFixture {
    /// Returns an explicit provider error.
    ///
    /// # Parameters
    /// - `self`: Fixture provider.
    /// - `_request`: Collection request.
    ///
    /// # Returns
    /// Never returns a collection page.
    ///
    /// # Errors
    /// Always returns the fixture capability error.
    fn list(&self, _request: &ListRequest) -> Result<ListResult, QueryError> {
        Err(QueryError::UnsupportedCapability {
            operation: scriberig_query::ReadOperation::List,
            capability: "fixture".to_owned(),
        })
    }

    /// Returns an explicit provider error.
    ///
    /// # Parameters
    /// - `self`: Fixture provider.
    /// - `_request`: Relation request.
    ///
    /// # Returns
    /// Never returns a traversal page.
    ///
    /// # Errors
    /// Always returns the fixture capability error.
    fn follow(&self, _request: &FollowRequest) -> Result<FollowResult, QueryError> {
        Err(QueryError::UnsupportedCapability {
            operation: scriberig_query::ReadOperation::Follow,
            capability: "fixture".to_owned(),
        })
    }
}

impl FindService for ReadFixture {
    /// Returns an explicit provider error.
    ///
    /// # Parameters
    /// - `self`: Fixture provider.
    /// - `_request`: Discovery request.
    ///
    /// # Returns
    /// Never returns discovery results.
    ///
    /// # Errors
    /// Always returns the fixture capability error.
    fn find(&self, _request: &FindRequest) -> Result<FindResult, QueryError> {
        Err(QueryError::UnsupportedCapability {
            operation: scriberig_query::ReadOperation::Find,
            capability: "fixture".to_owned(),
        })
    }
}

impl StructuredQueryService for ReadFixture {
    /// Returns an explicit provider error.
    ///
    /// # Parameters
    /// - `self`: Fixture provider.
    /// - `_specification`: Structural query request.
    ///
    /// # Returns
    /// Never returns query rows.
    ///
    /// # Errors
    /// Always returns the fixture capability error.
    fn query(&self, _specification: &QuerySpec) -> Result<QueryResult, QueryError> {
        Err(QueryError::UnsupportedCapability {
            operation: scriberig_query::ReadOperation::Query,
            capability: "fixture".to_owned(),
        })
    }
}

/// Records operation calls while returning semantic summaries.
struct OperationFixture {
    calls: Cell<usize>,
}

impl OperationFixture {
    /// Creates an empty operation recorder.
    ///
    /// # Returns
    /// A recorder with no calls.
    fn new() -> Self {
        Self {
            calls: Cell::new(0),
        }
    }

    /// Returns the next deterministic operation summary.
    ///
    /// # Parameters
    /// - `self`: Recorder to update.
    /// - `handle`: Stable result handle.
    ///
    /// # Returns
    /// A summary carrying the supplied handle.
    fn result(&self, handle: &str) -> OperationSummary {
        self.calls.set(self.calls.get() + 1);
        OperationSummary {
            handle: handle.to_owned(),
            snapshot: None,
            revision: None,
        }
    }
}

impl OperationProvider for OperationFixture {
    /// Records a diff request.
    ///
    /// # Parameters
    /// - `self`: Recorder provider.
    /// - `_context`: Application context.
    /// - `_request`: Diff request.
    ///
    /// # Returns
    /// Deterministic diff summary.
    ///
    /// # Errors
    /// This fixture does not fail.
    fn diff(
        &self,
        _context: &ApplicationContext,
        _request: &DiffRequest,
    ) -> Result<OperationSummary, scriberig_app::ApplicationError> {
        Ok(self.result("diff"))
    }

    /// Records a history request.
    ///
    /// # Parameters
    /// - `self`: Recorder provider.
    /// - `_context`: Application context.
    /// - `_request`: History request.
    ///
    /// # Returns
    /// Deterministic history summary.
    ///
    /// # Errors
    /// This fixture does not fail.
    fn history(
        &self,
        _context: &ApplicationContext,
        _request: &HistoryRequest,
    ) -> Result<OperationSummary, scriberig_app::ApplicationError> {
        Ok(self.result("history"))
    }

    /// Records a render request.
    ///
    /// # Parameters
    /// - `self`: Recorder provider.
    /// - `_context`: Application context.
    /// - `_request`: Render request.
    ///
    /// # Returns
    /// Deterministic render summary.
    ///
    /// # Errors
    /// This fixture does not fail.
    fn render(
        &self,
        _context: &ApplicationContext,
        _request: &RenderRequest,
    ) -> Result<OperationSummary, scriberig_app::ApplicationError> {
        Ok(self.result("render"))
    }

    /// Records a branch request.
    ///
    /// # Parameters
    /// - `self`: Recorder provider.
    /// - `_context`: Application context.
    /// - `_request`: Branch request.
    ///
    /// # Returns
    /// Deterministic branch summary.
    ///
    /// # Errors
    /// This fixture does not fail.
    fn branch(
        &self,
        _context: &ApplicationContext,
        _request: &BranchRequest,
    ) -> Result<OperationSummary, scriberig_app::ApplicationError> {
        Ok(self.result("branch"))
    }

    /// Records an apply request.
    ///
    /// # Parameters
    /// - `self`: Recorder provider.
    /// - `_context`: Application context.
    /// - `_request`: Apply request.
    ///
    /// # Returns
    /// Deterministic candidate summary.
    ///
    /// # Errors
    /// This fixture does not fail.
    fn apply(
        &self,
        _context: &ApplicationContext,
        _request: &ApplyRequest,
    ) -> Result<OperationSummary, scriberig_app::ApplicationError> {
        Ok(self.result("apply"))
    }

    /// Records a commit request.
    ///
    /// # Parameters
    /// - `self`: Recorder provider.
    /// - `_context`: Application context.
    /// - `_request`: Commit request.
    ///
    /// # Returns
    /// Deterministic commit summary.
    ///
    /// # Errors
    /// This fixture does not fail.
    fn commit(
        &self,
        _context: &ApplicationContext,
        _request: &CommitRequest,
    ) -> Result<OperationSummary, scriberig_app::ApplicationError> {
        Ok(self.result("commit"))
    }
}

/// Verifies all six non-read operations delegate to the canonical provider.
#[test]
fn operation_dispatch_covers_all_mutating_and_projection_operations() {
    let snapshot = SnapshotId::new([1; 32]);
    let root = RootName::new("main").expect("root");
    let backend = CompositeBackend::new(ReadFixture, OperationFixture::new());
    let context = ApplicationContext::empty();
    let requests = [
        ApplicationRequest::Diff(DiffRequest {
            from: snapshot,
            to: snapshot,
        }),
        ApplicationRequest::History(HistoryRequest {
            snapshot: SnapshotSelector::Exact(snapshot),
        }),
        ApplicationRequest::Render(RenderRequest {
            snapshot: SnapshotSelector::Exact(snapshot),
            view: "text".to_owned(),
        }),
        ApplicationRequest::Branch(BranchRequest {
            base: SnapshotSelector::Exact(snapshot),
            name: root,
        }),
        ApplicationRequest::Apply(ApplyRequest {
            base_snapshot: snapshot,
            change: "change".to_owned(),
        }),
        ApplicationRequest::Commit(CommitRequest {
            base_snapshot: snapshot,
            change: "change".to_owned(),
        }),
    ];
    for request in requests {
        assert!(matches!(
            scriberig_app::ApplicationBackend::execute(&backend, &context, &request),
            Ok(ApplicationResult::Diff(_)
                | ApplicationResult::History(_)
                | ApplicationResult::Render(_)
                | ApplicationResult::Branch(_)
                | ApplicationResult::Apply(_)
                | ApplicationResult::Commit(_),)
        ));
    }
}

/// Verifies every universal read variant reaches its typed provider.
#[test]
fn read_dispatch_preserves_structured_unsupported_capability() {
    let snapshot = SnapshotId::new([2; 32]);
    let entity = scriberig_core::EntityId::new([3; 16]);
    let subject = scriberig_query::EntityInSnapshot { entity, snapshot };
    let backend = CompositeBackend::new(ReadFixture, OperationFixture::new());
    let context = ApplicationContext::empty();
    let requests = [
        ApplicationRequest::Inspect(InspectRequest {
            target: scriberig_query::TargetRef::Snapshot(subject),
            detail: scriberig_query::DetailLevel::Identity,
            view: None,
        }),
        ApplicationRequest::List(ListRequest {
            subject,
            collection: scriberig_query::CollectionRef {
                owner: entity,
                collection_type: "children".to_owned(),
            },
            page: scriberig_query::PageRequest::default(),
            detail: scriberig_query::DetailLevel::Identity,
        }),
        ApplicationRequest::Follow(FollowRequest {
            subject,
            relation: scriberig_query::RelationRef {
                source: entity,
                relation_type: "calls".to_owned(),
                target: None,
            },
            direction: scriberig_query::FollowDirection::Outgoing,
            page: scriberig_query::PageRequest::default(),
            detail: scriberig_query::DetailLevel::Identity,
        }),
        ApplicationRequest::Find(FindRequest {
            scope: scriberig_query::QueryScope::Snapshot(snapshot),
            query: "x".to_owned(),
            strategy: scriberig_query::FindStrategy::ExactText,
            page: scriberig_query::PageRequest::default(),
            detail: scriberig_query::DetailLevel::Identity,
        }),
        ApplicationRequest::Query(QuerySpec {
            scope: scriberig_query::QueryScope::Snapshot(snapshot),
            expression: scriberig_query::QueryExpr::All,
            ordering: scriberig_query::QueryOrdering::StableDefault,
            page: scriberig_query::PageRequest::default(),
            detail: scriberig_query::DetailLevel::Identity,
        }),
    ];
    for request in requests {
        assert!(matches!(
            scriberig_app::ApplicationBackend::execute(&backend, &context, &request),
            Err(scriberig_app::ApplicationError::UnsupportedCapability { .. })
        ));
    }
}
