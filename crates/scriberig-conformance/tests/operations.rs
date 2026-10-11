//! Verifies operation lifecycle stability through the public client boundary.

use scriberig_client::Client;
use scriberig_conformance::fixture;
use scriberig_protocol::{OperationReference, OperationStatus};
use scriberig_server::InProcessTransport;

/// Verifies one operation handle reaches a terminal state through transport.
#[test]
fn operation_identity_and_terminal_state_survive_transport() {
    let client = Client::new(InProcessTransport::new(fixture()));
    let operation = client
        .operation_status(
            OperationReference {
                id: "op-1".to_owned(),
            }
            .id,
        )
        .expect("operation status");
    assert_eq!(operation.operation.reference.id, "op-1");
    assert_eq!(operation.operation.status, OperationStatus::Completed);
    assert_eq!(operation.operation.progress.expect("progress").percent, 100);
}
