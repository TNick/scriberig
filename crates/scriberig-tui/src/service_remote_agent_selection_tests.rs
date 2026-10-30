//! Verifies embedded agent selection normalization against the remote wire contract.

use super::*;

/// Keeps embedded semantic selections mapped to the shared remote wire contract.
#[test]
fn embedded_agent_selection_matches_shared_remote_normalization_fixture() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/agent-selection-normalization.json"
    ))
    .expect("shared agent selection normalization fixture");
    for case in fixture["cases"].as_array().expect("selection cases") {
        let semantic = &case["semantic"];
        let selection = scriberig_app::AgentSelection {
            project: Some(
                semantic["projectId"]
                    .as_str()
                    .expect("project identity")
                    .parse::<scriberig_core::ProjectId>()
                    .expect("valid project identity"),
            ),
            root: semantic["rootId"]
                .as_str()
                .expect("root identity")
                .parse::<EntityId>()
                .expect("valid root identity"),
            entity: semantic["entityId"]
                .as_str()
                .expect("entity identity")
                .parse::<EntityId>()
                .expect("valid entity identity"),
            snapshot: semantic["snapshotId"]
                .as_str()
                .expect("snapshot identity")
                .parse::<SnapshotId>()
                .expect("valid snapshot identity"),
        };

        assert_eq!(
            serde_json::to_value(protocol_agent_selection(&selection))
                .expect("serialize embedded selection mapping"),
            case["wire"]
        );
    }
}
