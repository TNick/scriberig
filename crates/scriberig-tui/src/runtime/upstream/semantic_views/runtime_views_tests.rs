//! Verifies the shared operation read model reaches the operations document.

use super::operations;
use crate::state::TuiState;
use crate::view::operations::OperationModel;
use scriberig_app::{OperationKind, OperationProgress, OperationRecord, OperationStatus};
use scriberig_core::OperationId;

/// Builds one shared operation record in the supplied lifecycle state.
///
/// - `id`: Stable operation identity.
/// - `kind`: Universal operation family.
/// - `status`: Current shared lifecycle state.
/// - `progress`: Shared progress observation, when the application reported one.
/// - `result_handles`: Stable result handles produced by the application.
///
/// # Returns
/// A shared application operation record.
fn record(
    id: &str,
    kind: OperationKind,
    status: OperationStatus,
    progress: Option<OperationProgress>,
    result_handles: Vec<String>,
) -> OperationRecord {
    OperationRecord {
        reference: OperationId::new(id).expect("operation id"),
        kind,
        base_snapshot: None,
        branch: None,
        status,
        progress,
        result_handles,
        agent_result: None,
        error: None,
    }
}

/// Verifies each operation row names the shared identity, family, and lifecycle state.
#[test]
fn operations_document_names_the_shared_operation_and_its_lifecycle() {
    let running = OperationModel::from_record(&record(
        "onboarding",
        OperationKind::Agent,
        OperationStatus::Running,
        Some(OperationProgress {
            status: OperationStatus::Running,
            completed: Some(2),
            total: Some(5),
            phase: Some("scanning sources".to_owned()),
        }),
        Vec::new(),
    ));
    let completed = OperationModel::from_record(&record(
        "history",
        OperationKind::History,
        OperationStatus::Completed,
        None,
        vec!["snapshot:result".to_owned()],
    ));
    let state = TuiState {
        operations: vec![running, completed],
        ..TuiState::default()
    };

    let output = operations(&state);

    assert!(output.contains("op_onboarding · Agent"));
    assert!(output.contains("Status"));
    assert!(output.contains("Running"));
    assert!(output.contains("scanning sources"));
    assert!(output.contains("op_history · History"));
    assert!(output.contains("Completed"));
    assert!(output.contains("Result: snapshot:result"));
}

/// Verifies no progress, result, or failure text appears unless shared state reported it.
#[test]
fn operations_document_omits_state_the_application_never_reported() {
    let queued = OperationModel::from_record(&record(
        "queued",
        OperationKind::Query,
        OperationStatus::Queued,
        None,
        Vec::new(),
    ));
    let state = TuiState {
        operations: vec![queued],
        ..TuiState::default()
    };

    let output = operations(&state);

    assert!(output.contains("op_queued · Query"));
    assert!(output.contains("Queued"));
    assert!(
        !output.contains("Result:"),
        "no result handle may be invented: {output}"
    );
    assert_eq!(
        output.matches("Status").count(),
        1,
        "exactly the shared lifecycle status may be reported: {output}"
    );
}
