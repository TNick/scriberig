//! Covers additional focused cases for the parent test module.

use super::*;

/// Verifies shared activity fixture covers output tool and sibling children.
#[test]
fn shared_activity_fixture_covers_output_tool_and_sibling_children() {
    let service = ApplicationService::new(Fixture);
    let root = service
        .start_local_agent(&AgentCommandRequest {
            session: EntityId::from_bytes([42; 16]),
            branch: None,
            anchor: None,
            prompt: "inspect and delegate".to_owned(),
            selection: None,
        })
        .expect("root operation");
    let architecture = service
        .start_local_child_agent(
            root.session,
            root.root_branch,
            "architecture",
            "map the architecture",
            "architecture result",
        )
        .expect("architecture operation");
    service
        .complete_local_child_agent(&architecture, "architecture result")
        .expect("architecture result");
    let implementation = service
        .start_local_child_agent(
            root.session,
            root.root_branch,
            "implementation",
            "implement the feature",
            "implementation result",
        )
        .expect("implementation operation");
    service
        .complete_local_child_agent(&implementation, "implementation result")
        .expect("implementation result");
    service
        .complete_local_agent_with_evidence(
            &root,
            "root result",
            Vec::new(),
            vec![
                scriberig_app::ToolExecutionEvidence {
                    ordinal: 1,
                    name: "delegate".to_owned(),
                    input: br#"{"purpose":"architecture"}"#.to_vec(),
                    presentation_input: Some(br#"{"purpose":"architecture"}"#.to_vec()),
                    presentation_label: None,
                    category: Some(scriberig_session::ToolActivityKind::BranchProducing),
                    output: br#"{"summary":"architecture result"}"#.to_vec(),
                    failed: false,
                    child_branch: Some(architecture.child_branch),
                },
                scriberig_app::ToolExecutionEvidence {
                    ordinal: 2,
                    name: "delegate".to_owned(),
                    input: br#"{"purpose":"implementation"}"#.to_vec(),
                    presentation_input: Some(br#"{"purpose":"implementation"}"#.to_vec()),
                    presentation_label: None,
                    category: Some(scriberig_session::ToolActivityKind::BranchProducing),
                    output: br#"{"summary":"implementation result"}"#.to_vec(),
                    failed: false,
                    child_branch: Some(implementation.child_branch),
                },
            ],
            Vec::new(),
        )
        .expect("root result");

    let session = service
        .read_session(
            &session_context(),
            &SessionRequest {
                session: root.session,
            },
        )
        .expect("durable session");
    assert_shared_activity_causality(
        &session,
        root.root_branch,
        &[architecture.child_branch, implementation.child_branch],
    );
}

/// Checks the durable parent and child activity relationships.
///
/// - `session`: Persisted activity graph to inspect.
/// - `root_branch`: Expected parent branch identity.
/// - `child_branches`: Expected child branch identities.
fn assert_shared_activity_causality(
    session: &scriberig_session::Session,
    root_branch: EntityId,
    child_branches: &[EntityId],
) {
    let parent = session
        .branches
        .iter()
        .find(|branch| branch.id == root_branch)
        .expect("parent branch");
    assert_eq!(parent.children, child_branches);
    assert!(parent.activity.iter().any(|item| matches!(
        item,
        scriberig_session::ExecutionActivity::ToolResult(id)
            if parent.tool_results.iter().any(|result| result.id == *id)
    )));
    for (child_branch, invocation) in child_branches.iter().zip(&parent.tool_invocations) {
        assert!(parent.activity.contains(
            &scriberig_session::ExecutionActivity::ChildBranchSpawned {
                branch: *child_branch,
                parent_tool_invocation: invocation.id,
            }
        ));
        let child = session
            .branches
            .iter()
            .find(|branch| branch.id == *child_branch)
            .expect("child branch");
        assert_eq!(child.parent, Some(root_branch));
        assert!(child.activity.iter().any(|item| matches!(
            item,
            scriberig_session::ExecutionActivity::AgentOutput(id)
                if child.turns.iter().any(|turn| turn.id == *id && turn.agent.is_some())
        )));
    }
    assert!(parent.activity.iter().any(|item| matches!(
        item,
        scriberig_session::ExecutionActivity::AgentOutput(id)
            if parent.turns.iter().any(|turn| turn.id == *id && turn.agent.is_some())
    )));
}
