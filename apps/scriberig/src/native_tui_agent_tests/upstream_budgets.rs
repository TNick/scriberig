//! Verifies recursive branch execution obeys `ScribeRig`'s depth budget.

use scriberig_agent_runtime::orchestration::RuntimeLimits;
use scriberig_session::BranchStatus;

use super::upstream_fixture::fixture::UpstreamFixture;
use super::upstream_fixture::provider::{Step, answer, step, tool_use};

/// Stops nested delegation at the configured maximum branch depth.
#[test]
fn nested_child_branches_stop_at_the_runtime_depth_limit() {
    let model = UpstreamFixture::catalog()
        .into_iter()
        .next()
        .expect("catalog row exists");
    let max_depth = RuntimeLimits::default().max_depth;
    let steps = (0..=max_depth)
        .map(|depth| {
            step(tool_use(
                &model,
                &format!("response-{depth}"),
                &format!("call-{depth}"),
                "delegate",
                serde_json::json!({
                    "purpose": format!("nested-{depth}"),
                    "prompt": format!("nested prompt {depth}"),
                    "capabilities": ["agent.delegate", "runtime_capabilities"],
                    "expected_result": "bounded nested result",
                }),
            ))
        })
        .collect::<Vec<Step>>();
    let fixture = UpstreamFixture::new(steps);
    let context = UpstreamFixture::context(&[]);
    let accepted = fixture
        .begin(&context, "exercise nested branch limits")
        .expect("accepted root operation");

    let _ = fixture.execute(&context, &accepted);

    let session = fixture.durable();
    assert_eq!(
        session.branches.len(),
        max_depth + 1,
        "the runtime refuses the child beyond the maximum nested depth"
    );
    assert!(
        fixture.request_count() > max_depth,
        "every accepted branch reached upstream model execution"
    );
    assert!(session.branches.iter().all(|branch| {
        branch.parent.is_none_or(|parent| {
            session
                .branches
                .iter()
                .any(|candidate| candidate.id == parent)
        })
    }));
    let deepest = session.branches.last().expect("deepest durable child");
    assert_eq!(deepest.status, BranchStatus::Completed);
    assert!(
        deepest.tool_results.iter().any(|result| result.failed),
        "delegation beyond the depth limit settles as a failed tool result"
    );
    fixture.cleanup();
}

/// Refuses child branches beyond the configured per-parent delegation budget.
#[test]
fn sibling_child_branches_stop_at_the_runtime_child_limit() {
    let model = UpstreamFixture::catalog()
        .into_iter()
        .next()
        .expect("catalog row exists");
    let max_children = RuntimeLimits::default().max_child_branches;
    let mut steps = Vec::with_capacity((max_children + 1) * 2);
    for child in 0..=max_children {
        steps.push(step(tool_use(
            &model,
            &format!("response-{child}"),
            &format!("call-{child}"),
            "delegate",
            serde_json::json!({
                "purpose": format!("sibling-{child}"),
                "prompt": format!("sibling prompt {child}"),
                "capabilities": ["runtime_capabilities"],
                "expected_result": "bounded sibling result",
            }),
        )));
        steps.push(step(answer(
            &model,
            &format!("child-{child}"),
            "child done",
        )));
    }
    let fixture = UpstreamFixture::new(steps);
    let context = UpstreamFixture::context(&[]);
    let accepted = fixture
        .begin(&context, "exercise sibling branch limits")
        .expect("accepted root operation");

    let _ = fixture.execute(&context, &accepted);

    let session = fixture.durable();
    assert_eq!(
        session.branches.len(),
        max_children + 1,
        "the rejected sibling does not create another durable branch"
    );
    let root = &session.branches[0];
    assert_eq!(root.children.len(), max_children);
    assert_eq!(
        root.tool_invocations
            .iter()
            .filter(|tool| tool.name == "delegate")
            .count(),
        max_children + 1
    );
    assert_eq!(
        root.tool_results
            .iter()
            .filter(|result| result.failed)
            .count(),
        1,
        "only the delegation beyond the configured limit fails"
    );
    assert!(
        session.branches[1..]
            .iter()
            .all(|branch| branch.status == BranchStatus::Completed)
    );
    fixture.cleanup();
}

/// Refuses semantic tool calls beyond the configured per-branch budget.
#[test]
fn semantic_tool_calls_stop_at_the_runtime_tool_limit() {
    let model = UpstreamFixture::catalog()
        .into_iter()
        .next()
        .expect("catalog row exists");
    let max_tool_calls = RuntimeLimits::default().max_tool_calls;
    let mut steps = Vec::with_capacity(max_tool_calls + 2);
    for call in 0..=max_tool_calls {
        steps.push(step(tool_use(
            &model,
            &format!("response-{call}"),
            &format!("call-{call}"),
            "runtime_capabilities",
            serde_json::json!({}),
        )));
    }
    steps.push(step(answer(&model, "final", "tool budget reached")));
    let fixture = UpstreamFixture::new(steps);
    let context = UpstreamFixture::context(&[]);
    let accepted = fixture
        .begin(&context, "exercise semantic tool limits")
        .expect("accepted root operation");

    let _ = fixture.execute(&context, &accepted);

    let session = fixture.durable();
    assert_eq!(
        session.branches.len(),
        1,
        "tools do not create child branches"
    );
    let root = &session.branches[0];
    assert_eq!(
        root.tool_invocations.len(),
        max_tool_calls + 1,
        "the over-budget call remains visible in durable branch evidence"
    );
    assert_eq!(
        root.tool_results
            .iter()
            .filter(|result| result.failed)
            .count(),
        1,
        "only the call beyond the configured limit fails"
    );
    fixture.cleanup();
}

/// Runs sibling child branches on independently resolved provider/model pairs.
#[test]
fn delegated_child_uses_its_explicit_upstream_execution_selection() {
    let models = UpstreamFixture::catalog();
    let parent_model = models.first().expect("parent catalog row exists");
    let child_model = models.get(1).expect("child catalog row exists");
    let fixture = UpstreamFixture::new(vec![
        step(tool_use(
            parent_model,
            "parent-response",
            "delegate-child",
            "delegate",
            serde_json::json!({
                "purpose": "alternate provider task",
                "prompt": "use the alternate model",
                "capabilities": ["runtime_capabilities"],
                "expected_result": "alternate child result",
                "execution": {
                    "provider_id": child_model.provider,
                    "model_id": child_model.id,
                },
            }),
        )),
        step(tool_use(
            parent_model,
            "parent-second-response",
            "delegate-local-child",
            "delegate",
            serde_json::json!({
                "purpose": "default provider task",
                "prompt": "use the inherited model",
                "capabilities": ["runtime_capabilities"],
                "expected_result": "default child result",
            }),
        )),
        step(answer(parent_model, "local-child", "local child done")),
        step(answer(parent_model, "parent-final", "parent completed")),
    ]);
    let context = UpstreamFixture::context(&[]);
    let accepted = fixture
        .begin(&context, "delegate to the alternate model")
        .expect("accepted root operation");

    fixture
        .execute(&context, &accepted)
        .expect("parent and both children complete");

    let session = fixture.durable();
    assert_eq!(session.branches.len(), 3);
    assert_eq!(session.execution.provider_id, None);
    assert_eq!(session.execution.model_id, None);
    let alternate_child = &session.branches[1];
    let alternate_invocation = alternate_child
        .model_invocations
        .first()
        .expect("alternate child model invocation persisted");
    let inherited_child = &session.branches[2];
    let inherited_invocation = inherited_child
        .model_invocations
        .first()
        .expect("inherited child model invocation persisted");
    assert_eq!(
        alternate_invocation.provider_id.as_deref(),
        Some("alternate-provider")
    );
    assert_eq!(
        alternate_invocation.provider_model_id.as_deref(),
        Some("alternate-model")
    );
    assert_eq!(inherited_invocation.provider_id.as_deref(), Some("local"));
    assert_eq!(
        inherited_invocation.provider_model_id.as_deref(),
        Some("fake-model")
    );
    assert_eq!(alternate_child.parent, inherited_child.parent);
    assert_ne!(
        alternate_invocation.provider_id, inherited_invocation.provider_id,
        "sibling branches may retain distinct upstream provider selections"
    );
    assert_eq!(fixture.provider.request_count(), 4);
    fixture.cleanup();
}

/// Rejects an unregistered child model before creating its durable branch.
#[test]
fn delegated_child_rejects_unknown_upstream_model_selection() {
    let parent_model = UpstreamFixture::catalog()
        .into_iter()
        .next()
        .expect("parent catalog row exists");
    let fixture = UpstreamFixture::new(vec![
        step(tool_use(
            &parent_model,
            "parent-response",
            "delegate-unknown",
            "delegate",
            serde_json::json!({
                "purpose": "unknown provider task",
                "prompt": "try an unregistered model",
                "capabilities": ["runtime_capabilities"],
                "expected_result": "no child is created",
                "execution": {
                    "provider_id": "missing-provider",
                    "model_id": "missing-model",
                },
            }),
        )),
        step(answer(&parent_model, "parent-final", "parent completed")),
    ]);
    let context = UpstreamFixture::context(&[]);
    let accepted = fixture
        .begin(&context, "reject an unknown child model")
        .expect("accepted root operation");

    fixture
        .execute(&context, &accepted)
        .expect("parent recovers from rejected delegation");

    let session = fixture.durable();
    assert_eq!(session.branches.len(), 1);
    assert_eq!(session.branches[0].children.len(), 0);
    assert!(
        session.branches[0]
            .tool_results
            .iter()
            .any(|result| result.failed)
    );
    assert_eq!(session.execution.provider_id, None);
    assert_eq!(session.execution.model_id, None);
    fixture.cleanup();
}
