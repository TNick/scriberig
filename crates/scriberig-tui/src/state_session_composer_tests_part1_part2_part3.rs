//! Covers additional focused cases for the parent test module.

use super::*;

/// Creates a safe model metadata fixture for plan assignment controls.
///
/// # Returns
/// A neutral upstream metadata snapshot for plan assignment fixtures.
fn model_metadata_snapshot() -> scriberig_protocol::PublicModelMetadataSnapshot {
    scriberig_protocol::PublicModelMetadataSnapshot {
        schema_version: "rpi-safe-model-snapshot-v1".to_owned(),
        api_id: "openai-completions".to_owned(),
        reasoning: false,
        input: vec![scriberig_protocol::PublicModelInputModality::Text],
        cost: scriberig_protocol::PublicModelCostSnapshot {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
            tiers: Vec::new(),
        },
        cost_configured: false,
        context_window: 4096,
        max_tokens: 1024,
    }
}

/// Verifies plan replan command emits shared revision operation.
#[test]
fn plan_replan_command_emits_shared_revision_operation() {
    let session_id = scriberig_core::EntityId::from_bytes([154; 16]);
    let branch_id = scriberig_core::EntityId::from_bytes([155; 16]);
    let plan_id = scriberig_core::EntityId::from_bytes([156; 16]);
    let mut plan = scriberig_session::Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        "Build parser",
    )
    .expect("plan");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("planned");
    let mut state = TuiState {
        session_snapshot: Some(scriberig_session::Session {
            id: session_id,
            branches: vec![scriberig_session::Branch::new(branch_id, "Root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        }),
        ..TuiState::default()
    };
    state.conversation_composer.draft = "/plan replan clarify parser tests".to_owned();

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(effects.as_slice(), [Effect::Replan(request)]
        if request.session == session_id
            && request.plan == plan_id
            && request.expected_revision == 1
            && request.reason == "clarify parser tests"
            && request.additions.len() == 1));
}

/// Inserts an accepted durable plan into the conversation tree by its identity.
#[test]
fn accepted_plan_result_rebuilds_structural_conversation_rows() {
    let session_id = scriberig_core::EntityId::from_bytes([141; 16]);
    let branch_id = scriberig_core::EntityId::from_bytes([142; 16]);
    let plan_id = scriberig_core::EntityId::from_bytes([143; 16]);
    let node_id = scriberig_core::EntityId::from_bytes([144; 16]);
    let session = scriberig_session::Session {
        id: session_id,
        branches: vec![scriberig_session::Branch::new(branch_id, "Root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let mut plan = scriberig_session::Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        "Build parser",
    )
    .expect("plan");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("planned");
    plan.add_node(scriberig_session::PlanNode {
        id: node_id,
        parent: Some(scriberig_session::PlanNodeParent::Branch(branch_id)),
        title: "Implement parser".to_owned(),
        purpose: "Implement the parser".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: scriberig_session::PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
        abandon_reason: None,
        last_failure: None,
    })
    .expect("node");
    let mut state = TuiState {
        session_snapshot: Some(session),
        ..TuiState::default()
    };

    reduce(&mut state, Action::PlanUpdated(plan));

    let conversation = state.conversation.expect("conversation projection");
    assert!(
        conversation
            .index_of(crate::view::conversation::ConversationSource::Plan(plan_id))
            .is_some()
    );
    assert!(
        conversation
            .index_of(crate::view::conversation::ConversationSource::PlanNode(
                node_id
            ))
            .is_some()
    );
    assert_eq!(state.session_snapshot.unwrap().plans[0].id, plan_id);
}

/// Assigns the selected plan node through the displayed model picker.
#[test]
fn assign_command_targets_only_the_selected_plan_node() {
    let session_id = scriberig_core::EntityId::from_bytes([151; 16]);
    let branch_id = scriberig_core::EntityId::from_bytes([152; 16]);
    let plan_id = scriberig_core::EntityId::from_bytes([153; 16]);
    let node_id = scriberig_core::EntityId::from_bytes([154; 16]);
    let mut plan = scriberig_session::Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        "Build parser",
    )
    .expect("plan");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("planned");
    plan.add_node(scriberig_session::PlanNode {
        id: node_id,
        parent: Some(scriberig_session::PlanNodeParent::Branch(branch_id)),
        title: "Implement parser".to_owned(),
        purpose: "Implement the parser".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: scriberig_session::PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
        abandon_reason: None,
        last_failure: None,
    })
    .expect("node");
    let session = scriberig_session::Session {
        id: session_id,
        branches: vec![scriberig_session::Branch::new(branch_id, "Root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let conversation = crate::view::conversation::ConversationModel::from_session(&session);
    let mut state = TuiState {
        session_snapshot: Some(session),
        conversation: Some(conversation),
        conversation_cursor: Some(crate::view::conversation::ConversationSource::PlanNode(
            node_id,
        )),
        model_library: crate::inspector::LoadState::Ready(
            scriberig_protocol::ModelLibraryResponse {
                provider_readiness: Vec::new(),
                models: vec![scriberig_protocol::PublicModelDescriptor {
                    id: "model-small".to_owned(),
                    display_name: "CPU model".to_owned(),
                    provider_id: "provider-cpu".to_owned(),
                    provider_model_id: "model-small".to_owned(),
                    thinking_levels: vec!["balanced".to_owned()],
                    metadata_snapshot: model_metadata_snapshot(),
                }],
            },
        ),
        ..TuiState::default()
    };
    state.conversation_composer.draft = "/assign".to_owned();
    reduce(&mut state, Action::InputSubmitted);
    assert!(state.plan_assignment_target.is_some());

    let effects = crate::state::state_presentation::activate_palette_control(
        &mut state,
        "assign-plan-model:model-small",
    )
    .expect("plan assignment selection");

    assert!(matches!(
        effects.as_slice(),
        [Effect::SetPlanNodeAssignment(request)]
            if request.plan == plan_id
                && request.node == node_id
                && request.assignment.as_ref().is_some_and(|value| {
                    value.provider_id == "provider-cpu"
                        && value.model_id == "model-small"
                        && value.thinking_level.as_deref() == Some("balanced")
                })
    ));
}
