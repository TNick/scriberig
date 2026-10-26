//! Verifies explicit migration of a retired session model choice.

use std::sync::Arc;

use rpi_ai::{Api, Model, ThinkingLevel, ThinkingLevelMap};
use scriberig_app::{
    AgentConsoleBackend, ApplicationBackend, ApplicationContext, ApplicationError,
    ApplicationRequest, ApplicationResult, ApplicationService, CapabilityContext, CapabilityId,
    LegacyExecutionMappingRequest, PlanAssignmentRequest,
};
use scriberig_core::EntityId;
use scriberig_session::{
    Branch, ContinuationAnchor, Plan, PlanNode, PlanNodeParent, PlanNodeStatus, Session,
    SessionExecutionSelection, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;

/// Backend fixture that rejects unrelated universal operations.
struct Fixture;

impl ApplicationBackend for Fixture {
    /// Returns unsupported-operation for requests outside the recovery test.
    ///
    /// - `_context`: Unused test context.
    /// - `_request`: Unused semantic request.
    ///
    /// # Returns
    /// Never returns a result.
    ///
    /// # Errors
    /// Always returns unsupported-operation.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "fixture request".to_owned(),
        })
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "This regression verifies current catalog resolution, CAS mutation, and stale mapping rejection across one recovered session."
)]
/// Maps a retired choice only after the exact current upstream model resolves.
fn operator_mapping_validates_current_pair_and_clears_legacy_choice() {
    let database = std::env::temp_dir().join(format!(
        "scriberig-app-legacy-recovery-{}.sqlite",
        EntityId::new()
    ));
    let store = Arc::new(SqliteSessionStore::open(&database).expect("open store"));
    let session_id = EntityId::new();
    let mut model = Model::new(
        "native-current",
        "Current model",
        Api::AnthropicMessages,
        "provider-current",
        "https://example.test",
    );
    model.thinking_level_map = Some(ThinkingLevelMap::from([
        (ThinkingLevel::Low, Some("low".to_owned())),
        (ThinkingLevel::Medium, Some("medium".to_owned())),
    ]));
    let service = ApplicationService::new(Fixture)
        .with_session_store(store.clone())
        .with_upstream_models(vec![model]);
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_model_selection").expect("capability")
        ]),
        ..ApplicationContext::empty()
    };
    let session = Session {
        id: session_id,
        branches: vec![Branch::new(EntityId::new(), "agent root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: SessionExecutionSelection {
            revision: 4,
            ..SessionExecutionSelection::default()
        },
    };
    store.save(&session).expect("save current session");
    let raw = serde_json::json!({
        "revision": 4,
        "model_profile": "retired-profile",
        "model": "retired-model",
        "thinking_level": "deep",
        "history": []
    });
    store
        .write_legacy_payload_for_test(
            session_id,
            &serde_json::to_vec(&serde_json::json!({
                "id": session.id,
                "branches": session.branches,
                "goals": [],
                "tasks": [],
                "plans": [],
                "execution": raw
            }))
            .expect("encode legacy record"),
        )
        .expect("write legacy fixture");

    let legacy = service
        .legacy_execution_records(&context, session_id)
        .expect("inspect preserved legacy choice");
    assert_eq!(legacy.len(), 1);
    assert_eq!(legacy[0].path, "/execution");
    let rejected = service
        .map_legacy_execution_selection(
            &context,
            &LegacyExecutionMappingRequest {
                session: session_id,
                expected_revision: 4,
                legacy_path: "/execution".to_owned(),
                provider_id: "provider-missing".to_owned(),
                model_id: "model-missing".to_owned(),
                thinking_level: None,
            },
        )
        .expect_err("unknown upstream pair must not map");
    assert_eq!(rejected.category(), "invalid_request");
    assert_eq!(
        service
            .legacy_execution_records(&context, session_id)
            .expect("legacy record survives rejection"),
        legacy
    );

    let mapped = service
        .map_legacy_execution_selection(
            &context,
            &LegacyExecutionMappingRequest {
                session: session_id,
                expected_revision: 4,
                legacy_path: "/execution".to_owned(),
                provider_id: "provider-current".to_owned(),
                model_id: "native-current".to_owned(),
                thinking_level: Some("low".to_owned()),
            },
        )
        .expect("map to validated current model");
    assert_eq!(mapped.revision, 5);
    assert_eq!(mapped.provider_id.as_deref(), Some("provider-current"));
    assert_eq!(mapped.model_id.as_deref(), Some("native-current"));
    assert_eq!(mapped.thinking_level.as_deref(), Some("low"));
    assert_eq!(
        service
            .legacy_execution_records(&context, session_id)
            .expect("legacy record is cleared")
            .len(),
        0
    );
    let reloaded = store
        .load(session_id)
        .expect("reload mapped session")
        .unwrap();
    assert_eq!(reloaded.execution, mapped);
    drop(store);
    let _ = std::fs::remove_file(database);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "This regression verifies exact historical plan assignment mapping, CAS freshness, and persisted recovery cleanup as one lifecycle."
)]
/// Maps a historical legacy assignment onto the current plan through an explicit CAS.
fn operator_maps_historical_legacy_plan_assignment() {
    let database = std::env::temp_dir().join(format!(
        "scriberig-app-legacy-plan-recovery-{}.sqlite",
        EntityId::new()
    ));
    let store = Arc::new(SqliteSessionStore::open(&database).expect("open store"));
    let session_id = EntityId::new();
    let root_id = EntityId::new();
    let plan_id = EntityId::new();
    let node_id = EntityId::new();
    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead { branch: root_id },
        "map legacy assignment",
    )
    .expect("create plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(PlanNodeParent::Branch(root_id)),
        title: "work".to_owned(),
        purpose: "execute work".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    })
    .expect("add node");
    let plan_revision = plan.revision;
    let session = Session {
        id: session_id,
        branches: vec![Branch::new(root_id, "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: SessionExecutionSelection::default(),
    };
    store.save(&session).expect("save session");
    let mut raw = serde_json::to_value(&session).expect("serialize legacy session");
    raw["plans"][0]["nodes"][0]["assignment"] = serde_json::json!({
        "profile": "retired-profile",
        "model": "retired-model",
        "thinking_level": "deep"
    });
    store
        .write_legacy_payload_for_test(
            session_id,
            &serde_json::to_vec(&raw).expect("encode legacy payload"),
        )
        .expect("write legacy assignment");
    let mut model = Model::new(
        "native-current",
        "Current model",
        Api::AnthropicMessages,
        "provider-current",
        "https://example.test",
    );
    model.thinking_level_map = Some(ThinkingLevelMap::from([
        (ThinkingLevel::Low, Some("low".to_owned())),
        (ThinkingLevel::Medium, Some("medium".to_owned())),
    ]));
    let service = ApplicationService::new(Fixture)
        .with_session_store(store.clone())
        .with_upstream_models(vec![model]);
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_plan").expect("plan capability"),
            CapabilityId::new("session_model_selection").expect("model capability"),
            CapabilityId::new("session_branch_tree").expect("session capability"),
        ]),
        ..ApplicationContext::empty()
    };
    let records = service
        .legacy_execution_records(&context, session_id)
        .expect("inspect legacy plan choice");
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].target.as_ref().map(|target| target.plan_id),
        Some(plan_id)
    );
    let mut historical = records[0].clone();
    historical
        .target
        .as_mut()
        .expect("stable plan target")
        .revision = Some(plan_revision.saturating_sub(1));
    store
        .write_legacy_payload_for_test(
            session_id,
            &serde_json::to_vec(&serde_json::json!({
                "version": 1,
                "session": session,
                "legacy_execution": [historical]
            }))
            .expect("encode historical record"),
        )
        .expect("write historical recovery envelope");
    let records = service
        .legacy_execution_records(&context, session_id)
        .expect("reload historical plan choice");
    assert_eq!(
        records[0]
            .target
            .as_ref()
            .and_then(|target| target.revision),
        Some(0)
    );
    let rejected = service
        .map_legacy_plan_assignment(
            &context,
            &scriberig_app::LegacyPlanAssignmentMappingRequest {
                session: session_id,
                legacy_path: records[0].path.clone(),
                expected_plan_revision: plan_revision,
                provider_id: "missing-provider".to_owned(),
                model_id: "missing-model".to_owned(),
                thinking_level: None,
            },
        )
        .expect_err("unknown model pair must be rejected");
    assert_eq!(rejected.category(), "invalid_request");
    assert_eq!(
        service
            .legacy_execution_records(&context, session_id)
            .expect("choice remains after rejection"),
        records
    );
    let intervening = service
        .set_plan_node_assignment(
            &context,
            &PlanAssignmentRequest {
                session: session_id,
                plan: plan_id,
                node: node_id,
                expected_revision: plan_revision,
                assignment: Some(scriberig_session::ModelAssignment {
                    provider_id: "provider-current".to_owned(),
                    model_id: "native-current".to_owned(),
                    thinking_level: None,
                }),
            },
        )
        .expect("intervening plan update");
    let stale = service
        .map_legacy_plan_assignment(
            &context,
            &scriberig_app::LegacyPlanAssignmentMappingRequest {
                session: session_id,
                legacy_path: records[0].path.clone(),
                expected_plan_revision: plan_revision,
                provider_id: "provider-current".to_owned(),
                model_id: "native-current".to_owned(),
                thinking_level: Some("low".to_owned()),
            },
        )
        .expect_err("stale plan revision must be rejected");
    assert_eq!(stale.category(), "conflict");
    assert_eq!(
        service
            .legacy_execution_records(&context, session_id)
            .expect("choice remains after stale plan revision"),
        records
    );
    let updated = service
        .map_legacy_plan_assignment(
            &context,
            &scriberig_app::LegacyPlanAssignmentMappingRequest {
                session: session_id,
                legacy_path: records[0].path.clone(),
                expected_plan_revision: intervening.revision,
                provider_id: "provider-current".to_owned(),
                model_id: "native-current".to_owned(),
                thinking_level: Some("low".to_owned()),
            },
        )
        .expect("map historical assignment onto the current plan");
    assert_eq!(updated.revision, intervening.revision + 1);
    assert_eq!(
        updated.nodes[0]
            .assignment
            .as_ref()
            .map(|item| item.model_id.as_str()),
        Some("native-current")
    );
    assert_eq!(
        service
            .legacy_execution_records(&context, session_id)
            .expect("mapped record cleared")
            .len(),
        0
    );
    drop(store);
    let _ = std::fs::remove_file(database);
}
