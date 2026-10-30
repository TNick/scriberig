//! Verifies typed reference, request, and CAD anchor protocol contracts.

use super::*;

/// Verifies stable references and snapshot-aware requests round trip.
#[test]
fn references_and_requests_round_trip() {
    let entity = EntityReference {
        id: "entity-1".to_owned(),
    };
    round_trip(&entity);
    round_trip(&InspectEntityRequest {
        entity,
        snapshot: SnapshotSelector::Current,
        detail: request::DetailLevel::Deep,
    });
    round_trip(&QueryRequest {
        specification: serde_json::json!({"scope": "snapshot"}),
    });
    round_trip(&ListRequest {
        specification: serde_json::json!({"collection": "snapshot.entities"}),
    });
    round_trip(&CommandRequest {
        kind: "apply".to_owned(),
        payload: serde_json::json!({"change": "change-1"}),
    });
    round_trip(&DomainOperationCommand {
        capability: "code.add_function".to_owned(),
        entity: EntityReference {
            id: "entity-1".to_owned(),
        },
        snapshot: SnapshotReference {
            id: "snapshot-1".to_owned(),
        },
        revision: None,
        branch: Some("root_working".to_owned()),
        projection_anchor: Some(ProjectionAnchor::SpreadsheetCell {
            worksheet_id: "worksheet-1".to_owned(),
            row: 3,
            column: 2,
        }),
        arguments: [("name".to_owned(), serde_json::json!("greet"))]
            .into_iter()
            .collect(),
    });
    round_trip(&DomainOperationCommand {
        capability: "cad.update_text".to_owned(),
        entity: EntityReference {
            id: "entity-cad-line".to_owned(),
        },
        snapshot: SnapshotReference {
            id: "snapshot-cad".to_owned(),
        },
        revision: None,
        branch: None,
        projection_anchor: Some(ProjectionAnchor::CadBounds {
            drawing_id: "drawing-1".to_owned(),
            space: "model".to_owned(),
            min_x: "-1.25".to_owned(),
            min_y: "0".to_owned(),
            max_x: "4.75".to_owned(),
            max_y: "8".to_owned(),
        }),
        arguments: std::collections::BTreeMap::new(),
    });
    round_trip(&AgentStartRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        prompt: "inspect the selected branch".to_owned(),
        selection: None,
    });
    round_trip(&AgentNewSessionRequest {
        prompt: "create a conversation".to_owned(),
        selection: None,
    });
    round_trip(&Request::AgentNewSession(AgentNewSessionRequest {
        prompt: "create a conversation".to_owned(),
        selection: None,
    }));
    round_trip(&AgentContinueRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        branch: BranchReference {
            id: "branch-1".to_owned(),
        },
        prompt: "continue".to_owned(),
        selection: None,
    });
    round_trip(&AgentContinueAtRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        anchor: SessionContinuationAnchor::AssistantOutput {
            branch: "branch-1".to_owned(),
            turn: "turn-7".to_owned(),
        },
        prompt: "continue from this exact answer".to_owned(),
        selection: None,
    });
    plan_contracts::session_plan_contracts_round_trip();
    tests_additional::model_contracts_round_trip();
}

/// Rejects malformed or oversized CAD projection bounds.
#[test]
fn cad_projection_bounds_validate_descriptor_limits() {
    let valid = ProjectionAnchor::CadBounds {
        drawing_id: "drawing-1".to_owned(),
        space: "model".to_owned(),
        min_x: "-1.25".to_owned(),
        min_y: "0".to_owned(),
        max_x: "4.75".to_owned(),
        max_y: "8".to_owned(),
    };
    assert!(valid.validate().is_ok());

    let oversized = ProjectionAnchor::CadBounds {
        drawing_id: "d".repeat(129),
        space: "model".to_owned(),
        min_x: "0".to_owned(),
        min_y: "0".to_owned(),
        max_x: "1".to_owned(),
        max_y: "1".to_owned(),
    };
    assert!(oversized.validate().is_err());
}
