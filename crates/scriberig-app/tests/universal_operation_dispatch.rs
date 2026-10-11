//! Verifies universal application dispatch.

#[path = "universal_operation_support.rs"]
mod support;
use scriberig_app::*;
use scriberig_core::*;
use support::*;
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
            snapshot: SnapshotSelector::NamedRoot(RootName::new("main").expect("root")),
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
            branch: None,
            change: "change".to_owned(),
        }),
        ApplicationRequest::Commit(CommitRequest {
            base_snapshot: snapshot,
            branch: None,
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
