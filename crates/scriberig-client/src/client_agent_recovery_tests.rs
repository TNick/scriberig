//! Verifies protocol client requests for legacy execution recovery.

use super::*;

/// Verifies legacy recovery methods preserve the exact record and mapping inputs.
#[test]
fn legacy_recovery_methods_preserve_exact_requests() {
    let observed = Arc::new(Mutex::new(None));
    let client = Client::new(FixtureTransport {
        response: Response::LegacyExecutionRecords(vec![
            scriberig_protocol::LegacyExecutionRecordView {
                path: "/execution".to_owned(),
                value: serde_json::json!({"model": "old-name"}),
                target: None,
            },
        ]),
        request: Arc::clone(&observed),
    });
    let records = client
        .legacy_execution_records("session-a")
        .expect("legacy records response");
    assert_eq!(records[0].path, "/execution");
    assert_eq!(
        observed.lock().expect("observed request").as_ref(),
        Some(&Request::LegacyExecutionRecords(
            scriberig_protocol::LegacyExecutionRecordsRequest {
                session: SessionReference {
                    id: "session-a".to_owned(),
                },
            }
        ))
    );

    let observed = Arc::new(Mutex::new(None));
    let client = Client::new(FixtureTransport {
        response: Response::SessionExecution(SessionExecutionSelection {
            revision: 4,
            provider_id: Some("provider-a".to_owned()),
            model_id: Some("model-a".to_owned()),
            thinking_level: None,
            history: Vec::new(),
        }),
        request: Arc::clone(&observed),
    });
    client
        .map_legacy_execution_selection("session-a", 3, "/execution", "provider-a", "model-a", None)
        .expect("mapped selection response");
    assert_eq!(
        observed.lock().expect("observed request").as_ref(),
        Some(&Request::LegacyExecutionMapping(
            scriberig_protocol::LegacyExecutionMappingRequest {
                session: SessionReference {
                    id: "session-a".to_owned(),
                },
                expected_revision: 3,
                legacy_path: "/execution".to_owned(),
                provider_id: "provider-a".to_owned(),
                model_id: "model-a".to_owned(),
                thinking_level: None,
            }
        ))
    );

    let observed = Arc::new(Mutex::new(None));
    let client = Client::new(FixtureTransport {
        response: Response::Plan(empty_plan()),
        request: Arc::clone(&observed),
    });
    client
        .map_legacy_plan_assignment(
            "session-a",
            "/plans/0/nodes/0/assignment",
            4,
            "provider-a",
            "model-a",
            None,
        )
        .expect("mapped plan response");
    assert_eq!(
        observed.lock().expect("observed request").as_ref(),
        Some(&Request::LegacyPlanAssignmentMapping(
            scriberig_protocol::LegacyPlanAssignmentMappingRequest {
                session: SessionReference {
                    id: "session-a".to_owned(),
                },
                legacy_path: "/plans/0/nodes/0/assignment".to_owned(),
                expected_plan_revision: 4,
                provider_id: "provider-a".to_owned(),
                model_id: "model-a".to_owned(),
                thinking_level: None,
            }
        ))
    );
}
