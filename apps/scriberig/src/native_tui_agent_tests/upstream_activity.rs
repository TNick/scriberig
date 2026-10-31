//! Verifies durable execution activity keeps `ScribeRig` ordering, identity,
//! and single settlement on the upstream-backed branch path.
//!
//! Upstream owns provider streaming and ordinary tool-call settlement. These
//! tests pin that `ScribeRig` records one activity sequence per branch with its
//! own identities, so streamed deltas stay transient, a settled tool call stays
//! correlated to the model invocation that requested it, and durable activity
//! identity never adopts a provider-owned identity.

use rpi_ai::providers::faux::faux_text;
use rpi_ai::types::{AssistantMessage, StopReason};

use super::upstream_fixture::fixture::UpstreamFixture;
use super::upstream_fixture::provider::{Step, answer, step, tool_use};

/// One tool-call request followed by one final answer for the fixture model.
///
/// # Returns
/// The scripted provider steps for a model-tool-model branch.
fn tool_then_answer() -> Vec<Step> {
    let model = UpstreamFixture::catalog()
        .into_iter()
        .next()
        .expect("catalog row exists");
    vec![
        step(tool_use(
            &model,
            "response-1",
            "call-1",
            "runtime_capabilities",
            serde_json::json!({}),
        )),
        step(answer(&model, "response-2", "final upstream answer")),
    ]
}

/// Returns the durable activity kinds of one branch in recorded order.
///
/// - `branch`: Durable branch whose activity list is ordered.
///
/// # Returns
/// The activity kinds in durable order.
fn activity_kinds(branch: &scriberig_session::Branch) -> Vec<String> {
    branch
        .activity
        .iter()
        .map(|activity| match activity {
            scriberig_session::ExecutionActivity::HumanInput(_) => "human".to_owned(),
            scriberig_session::ExecutionActivity::ModelInvocation(_) => "model".to_owned(),
            scriberig_session::ExecutionActivity::ModelOutput(_) => "output".to_owned(),
            scriberig_session::ExecutionActivity::ToolInvocation(_) => "tool".to_owned(),
            scriberig_session::ExecutionActivity::ToolResult(_) => "result".to_owned(),
            scriberig_session::ExecutionActivity::ChildBranch(_) => "child".to_owned(),
            scriberig_session::ExecutionActivity::ChildBranchSpawned { .. } => {
                "child-spawned".to_owned()
            }
            scriberig_session::ExecutionActivity::BranchResult(_) => "branch-result".to_owned(),
            scriberig_session::ExecutionActivity::AgentOutput(_) => "agent-output".to_owned(),
        })
        .collect()
}

/// Pins the exact durable activity order for one model-tool-model branch.
#[test]
fn upstream_model_and_tool_activity_keeps_scribe_rig_order_and_correlation() {
    let fixture = UpstreamFixture::new(tool_then_answer());
    let handle = fixture
        .submit(&UpstreamFixture::context(&[]), "review the branch")
        .expect("upstream-backed root turn");

    let session = fixture.durable();
    let root = &session.branches[0];
    assert_eq!(
        activity_kinds(root),
        vec![
            "human",
            "model",
            "tool",
            "result",
            "model",
            "agent-output",
            "branch-result"
        ],
        "the model request that produced a tool call must precede that tool call"
    );
    assert_eq!(root.model_invocations.len(), 2);
    assert!(
        root.model_invocations
            .iter()
            .all(|invocation| { invocation.operation.as_ref() == Some(&handle.operation) })
    );
    assert_eq!(root.tool_invocations.len(), 1);
    assert_eq!(root.tool_results.len(), 1);
    assert_eq!(
        root.model_invocations
            .iter()
            .map(|invocation| invocation.ordinal)
            .collect::<Vec<_>>(),
        vec![0, 2],
        "model requests and settled tool calls share one ScribeRig activity sequence"
    );
    assert_eq!(
        root.tool_invocations[0].parent_model_invocation,
        Some(root.model_invocations[0].id),
        "the settled tool call stays correlated to the invocation that requested it"
    );
    assert_eq!(
        root.tool_results[0].invocation, root.tool_invocations[0].id,
        "the tool result references the ScribeRig tool-invocation identity"
    );
    assert!(
        root.model_invocations
            .iter()
            .all(|invocation| invocation.branch == root.id),
        "every model invocation is correlated to its ScribeRig branch"
    );
    fixture.cleanup();
}

/// Keeps provider-owned identities out of durable activity identity.
#[test]
fn durable_activity_identity_never_reuses_an_upstream_identity() {
    let fixture = UpstreamFixture::new(tool_then_answer());
    let handle = fixture
        .submit(&UpstreamFixture::context(&[]), "review the branch")
        .expect("upstream-backed root turn");

    let session = fixture.durable();
    let root = &session.branches[0];
    assert_eq!(handle.session, fixture.session);
    assert_eq!(handle.branch, Some(root.id));
    assert_eq!(fixture.session, session.id);
    assert!(
        "upstream-response-1"
            .parse::<scriberig_core::EntityId>()
            .is_err(),
        "provider-owned response identifiers are not valid Scriberig session or branch IDs"
    );
    let activity = serde_json::to_string(&root.activity).expect("serialize activity");
    for upstream_identity in ["upstream-response-1", "upstream-response-2", "call-1"] {
        assert!(
            !activity.contains(upstream_identity),
            "durable activity identity must not carry the upstream identity {upstream_identity}"
        );
    }
    assert_eq!(
        root.activity.len(),
        root.model_invocations.len() + root.tool_invocations.len() + root.tool_results.len() + 3,
        "activity references one ScribeRig identity per recorded semantic event"
    );
    assert!(
        root.tool_invocations[0]
            .parent_model_invocation
            .is_some_and(|parent| root.model_invocations.iter().any(|item| item.id == parent)),
        "tool correlation points at a durable ScribeRig model invocation"
    );
    // The provider call identity is request content only, never a session
    // identity: it re-enters as model-visible context in the next request.
    let second_request = root.model_invocations[1]
        .request
        .as_ref()
        .expect("captured second request");
    assert!(
        serde_json::to_string(&second_request)
            .expect("serialize request")
            .contains("call-1"),
        "the provider call identity stays inside the captured request provenance"
    );
    fixture.cleanup();
}

/// Prevents an unadvertised provider tool name from reaching semantic dispatch.
#[test]
fn upstream_rejects_unadvertised_provider_tool_names() {
    let model = UpstreamFixture::catalog()
        .into_iter()
        .next()
        .expect("catalog row exists");
    let fixture = UpstreamFixture::new(vec![
        step(tool_use(
            &model,
            "unknown-tool",
            "unknown-call",
            "unadvertised_semantic_operation",
            serde_json::json!({}),
        )),
        step(answer(&model, "recovered", "unknown tool was rejected")),
    ]);
    let handle = fixture
        .submit(
            &UpstreamFixture::context(&[]),
            "try an unadvertised operation",
        )
        .expect("upstream loop returns a bounded tool error to the provider");

    let durable = fixture.durable();
    let root = &durable.branches[0];
    assert_eq!(
        root.turns
            .iter()
            .find_map(|turn| turn.agent.as_ref())
            .map(|output| output.text.as_str()),
        Some("unknown tool was rejected")
    );
    assert_eq!(root.id, handle.branch.expect("accepted Scriberig branch"));
    assert_eq!(fixture.provider.request_count(), 2);
    assert!(
        root.tool_invocations.is_empty(),
        "an unknown provider tool name must not enter Scriberig semantic dispatch"
    );
    fixture.cleanup();
}

/// Settles streamed deltas into exactly one durable agent output.
#[test]
fn streamed_deltas_settle_once_into_the_final_agent_output() {
    let model = UpstreamFixture::catalog()
        .into_iter()
        .next()
        .expect("catalog row exists");
    let streamed = AssistantMessage {
        content: vec![
            faux_text("final "),
            faux_text("upstream "),
            faux_text("answer"),
        ],
        response_id: Some("upstream-response-1".to_owned()),
        stop_reason: StopReason::Stop,
        ..AssistantMessage::empty(
            model.api.clone(),
            model.provider.clone(),
            model.id.clone(),
            0,
        )
    };
    let fixture = UpstreamFixture::new(vec![step(streamed)]);
    fixture
        .submit(&UpstreamFixture::context(&[]), "review the branch")
        .expect("upstream-backed root turn");

    let session = fixture.durable();
    let root = &session.branches[0];
    assert_eq!(
        root.activity
            .iter()
            .filter(|activity| matches!(
                activity,
                scriberig_session::ExecutionActivity::AgentOutput(_)
            ))
            .count(),
        1,
        "one branch turn records its final assistant output once"
    );
    assert_eq!(
        root.activity
            .iter()
            .filter(|activity| matches!(
                activity,
                scriberig_session::ExecutionActivity::BranchResult(_)
            ))
            .count(),
        1,
        "one branch turn records one durable branch result"
    );
    assert_eq!(
        root.model_invocations.len(),
        1,
        "streamed deltas must not create one model invocation per delta"
    );
    let outputs = root
        .turns
        .iter()
        .filter_map(|turn| turn.agent.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].text, "final upstream answer");
    fixture.cleanup();
}
