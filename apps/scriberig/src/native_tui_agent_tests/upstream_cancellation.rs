//! Verifies branch-local cancellation produces structured durable outcomes on
//! the upstream-backed execution path.
//!
//! Upstream owns loop and provider cancellation mechanics; `ScribeRig` owns the
//! branch-local token, the durable branch outcome, and the retained request and
//! tool evidence. The synchronous `inspect` semantic query stays outside this
//! contract because the application query boundary accepts no cancellation
//! token, so cancellation stops the loop and provider request but cannot
//! interrupt an inspect query once it has started.

use scriberig_app::ApplicationError;
use scriberig_session::BranchStatus;

use super::upstream_fixture::fixture::UpstreamFixture;
use super::upstream_fixture::provider::{Step, step, tool_use};

/// Stops a pending provider request and records one structured failed outcome.
#[test]
fn cancelled_root_branch_keeps_a_structured_durable_outcome() {
    let fixture = UpstreamFixture::new(vec![Step::HoldUntilCancelled]);
    let context = UpstreamFixture::context(&[]);
    let accepted = fixture
        .begin(&context, "hold the branch")
        .expect("accepted root operation");
    fixture.cancel(&accepted);

    let failure = fixture
        .execute(&context, &accepted)
        .expect_err("cancelled root turn");
    assert!(
        matches!(failure, ApplicationError::OperationFailed { .. }),
        "cancelled root turn: {failure:?}"
    );
    assert!(
        failure.to_string().contains("cancel"),
        "the public failure stays in the cancellation category: {failure:?}"
    );
    assert_eq!(
        fixture.request_count(),
        1,
        "the cancelled turn attempted exactly one provider request"
    );

    let session = fixture.durable();
    let root = &session.branches[0];
    assert_eq!(root.status, BranchStatus::Abandoned);
    let outcome = root.result.as_ref().expect("structured branch outcome");
    assert_eq!(outcome.branch, root.id);
    assert_eq!(outcome.status, BranchStatus::Abandoned);
    assert!(
        outcome
            .summary
            .as_deref()
            .is_some_and(|summary| summary.contains("cancel")),
        "the durable outcome names the cancellation: {outcome:?}"
    );
    assert!(
        matches!(
            outcome.result_kind.as_deref(),
            Some("agent-cancelled" | "agent-failure")
        ),
        "the outcome kind distinguishes a cancelled agent branch: {outcome:?}"
    );
    assert!(
        root.model_invocations
            .iter()
            .any(|invocation| invocation.request.is_some()),
        "the prepared request projection stays durable with the abandoned branch"
    );
    assert!(
        !root.activity.is_empty(),
        "the abandoned branch keeps its durable activity record"
    );
    fixture.cleanup();
}

/// Cancels one delegated child and keeps both branch outcomes structured.
///
/// The cancellation is raised when the child's own provider request starts, so
/// the child branch is already durable and executing before it is stopped.
#[test]
fn cancelled_child_branch_keeps_a_structured_durable_outcome() {
    let model = UpstreamFixture::catalog()
        .into_iter()
        .next()
        .expect("catalog row exists");
    let fixture = UpstreamFixture::new(vec![
        step(tool_use(
            &model,
            "response-1",
            "call-1",
            "delegate",
            serde_json::json!({
                "purpose": "architecture",
                "prompt": "architecture private prompt",
                "capabilities": ["runtime_capabilities"],
                "expected_result": "compact architecture result",
            }),
        )),
        Step::HoldUntilCancelled,
    ]);
    let context = UpstreamFixture::context(&[]);
    let accepted = fixture
        .begin(&context, "delegate the work")
        .expect("accepted root operation");
    let cancellation = fixture
        .service
        .operations()
        .cancellation_flag(&accepted.operation)
        .expect("operation cancellation flag");
    fixture.provider.set_hook(move |index| {
        if index == 2 {
            cancellation.store(true, std::sync::atomic::Ordering::Release);
        }
    });

    let _ = fixture.execute(&context, &accepted);

    let session = fixture.durable();
    assert_eq!(session.branches.len(), 2, "the child branch is durable");
    let root = &session.branches[0];
    let child = &session.branches[1];
    assert_eq!(
        child.parent,
        Some(root.id),
        "the child stays linked to the canonical parent branch"
    );
    assert_eq!(
        child.status,
        BranchStatus::Abandoned,
        "the cancelled child settles as an abandoned branch"
    );
    let outcome = child.result.as_ref().expect("structured child outcome");
    assert_eq!(outcome.status, BranchStatus::Abandoned);
    assert_eq!(outcome.result_kind.as_deref(), Some("agent-child-failure"));
    assert!(
        outcome
            .summary
            .as_deref()
            .is_some_and(|summary| !summary.is_empty()),
        "the child outcome carries a safe structured summary"
    );
    assert_eq!(
        child.model_invocations.len(),
        1,
        "the child was durable before its first model invocation and keeps it"
    );
    assert!(
        child.model_invocations[0]
            .request
            .as_ref()
            .is_some_and(|request| request
                .messages
                .iter()
                .any(|message| message.content.contains("architecture private prompt"))),
        "the child's prepared request projection stays durable with the abandoned child"
    );
    assert!(
        root.activity.iter().any(|activity| {
            matches!(activity,
                scriberig_session::ExecutionActivity::ChildBranch(id)
                | scriberig_session::ExecutionActivity::ChildBranchSpawned { branch: id, .. }
                    if *id == child.id)
        }),
        "the parent keeps the durable link to the cancelled child branch"
    );
    assert_eq!(
        root.status,
        BranchStatus::Abandoned,
        "the parent branch also settles with a structured outcome"
    );
    assert!(
        root.result
            .as_ref()
            .is_some_and(|result| result.status == BranchStatus::Abandoned),
        "the parent outcome is structured rather than a lost branch"
    );
    fixture.cleanup();
}
