//! Verifies durable plan evolution HTTP operation routes.

use super::*;

/// Routes the public next-step operation through the shared plan dispatcher.
#[test]
fn typed_plan_next_step_route_dispatches_shared_operation() {
    let state = HttpState {
        service: Fixture,
        principal: scriberig_app::Principal::new("test-user"),
        allowed_origin: None,
        project_store: None,
        allowed_project_roots: Vec::new(),
        plan_lifecycle_handler: None,
        provider_readiness_handler: None,
        fixture: DevelopmentFixture {
            root_entity: EntityId::from_bytes([1; 16]),
            child_entities: Vec::new(),
            snapshot: SnapshotId::new([1; 32]),
        },
    };
    let request = scriberig_protocol::SessionPlanNextStepRequest {
        session: scriberig_protocol::SessionReference {
            id: EntityId::from_bytes([2; 16]).to_string(),
        },
        plan: EntityId::from_bytes([3; 16]).to_string(),
        expected_revision: 9,
        title: "next".to_owned(),
        purpose: "continue work".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        assignment: None,
    };
    let body = serde_json::to_string(&request).expect("encode next-step request");
    let response = dispatch(&state, "plan_next_step", &body, "plan-next".to_owned())
        .expect("typed protocol response");
    assert_eq!(response["correlation_id"], "plan-next");
    assert_eq!(
        response["response"]["Error"]["code"],
        "UnsupportedOperation"
    );
}

/// Routes public node decomposition through the shared plan dispatcher.
#[test]
fn typed_plan_decompose_route_dispatches_shared_operation() {
    let state = HttpState {
        service: Fixture,
        principal: scriberig_app::Principal::new("test-user"),
        allowed_origin: None,
        project_store: None,
        allowed_project_roots: Vec::new(),
        plan_lifecycle_handler: None,
        provider_readiness_handler: None,
        fixture: DevelopmentFixture {
            root_entity: EntityId::from_bytes([1; 16]),
            child_entities: Vec::new(),
            snapshot: SnapshotId::new([1; 32]),
        },
    };
    let request = scriberig_protocol::SessionPlanDecomposeRequest {
        session: scriberig_protocol::SessionReference {
            id: EntityId::from_bytes([2; 16]).to_string(),
        },
        plan: EntityId::from_bytes([3; 16]).to_string(),
        parent: EntityId::from_bytes([4; 16]).to_string(),
        expected_revision: 10,
        children: vec![scriberig_protocol::SessionPlanChildDraft {
            title: "child".to_owned(),
            purpose: "complete one part".to_owned(),
            expected_result: None,
            semantic_scope: Vec::new(),
            assignment: None,
        }],
        required: true,
    };
    let body = serde_json::to_string(&request).expect("encode decomposition request");
    let response = dispatch(&state, "plan_decompose", &body, "plan-decompose".to_owned())
        .expect("typed protocol response");
    assert_eq!(response["correlation_id"], "plan-decompose");
    assert_eq!(
        response["response"]["Error"]["code"],
        "UnsupportedOperation"
    );
}

/// Routes typed semantic mutations through the shared plan dispatcher.
#[test]
fn typed_plan_semantic_mutation_route_dispatches_shared_operation() {
    let state = HttpState {
        service: Fixture,
        principal: scriberig_app::Principal::new("test-user"),
        allowed_origin: None,
        project_store: None,
        allowed_project_roots: Vec::new(),
        plan_lifecycle_handler: None,
        provider_readiness_handler: None,
        fixture: DevelopmentFixture {
            root_entity: EntityId::from_bytes([1; 16]),
            child_entities: Vec::new(),
            snapshot: SnapshotId::new([1; 32]),
        },
    };
    let request = scriberig_protocol::SessionPlanSemanticMutationRequest {
        session: scriberig_protocol::SessionReference {
            id: EntityId::from_bytes([2; 16]).to_string(),
        },
        plan: EntityId::from_bytes([3; 16]).to_string(),
        expected_revision: 11,
        mutation: scriberig_protocol::SessionPlanSemanticMutation::UpdateSummary {
            node: EntityId::from_bytes([4; 16]).to_string(),
            text: "Current progress".to_owned(),
            sources: Vec::new(),
        },
    };
    let body = serde_json::to_string(&request).expect("encode semantic mutation request");
    let response = dispatch(
        &state,
        "plan_semantic_mutation",
        &body,
        "plan-semantic".to_owned(),
    )
    .expect("typed protocol response");
    assert_eq!(response["correlation_id"], "plan-semantic");
    assert_eq!(
        response["response"]["Error"]["code"],
        "UnsupportedOperation"
    );
}
