//! Verifies exact project and workspace scope in public agent selection requests.

use super::{AgentSelection, EntityReference, SnapshotReference};

/// Decodes the web client's exact project/root/entity/snapshot wire shape.
#[test]
fn web_agent_selection_wire_preserves_project_root_and_exact_target() {
    let selection: AgentSelection = serde_json::from_value(serde_json::json!({
        "project_id": "project-1",
        "root": { "id": "ent_workspace_root" },
        "entity": { "id": "ent_cell" },
        "snapshot": { "id": "snp_exact" }
    }))
    .expect("typed semantic agent selection");

    assert_eq!(selection.project_id.as_deref(), Some("project-1"));
    assert_eq!(
        selection.root,
        Some(EntityReference {
            id: "ent_workspace_root".to_owned()
        })
    );
    assert_eq!(
        selection.entity,
        EntityReference {
            id: "ent_cell".to_owned()
        }
    );
    assert_eq!(
        selection.snapshot,
        SnapshotReference {
            id: "snp_exact".to_owned()
        }
    );
}
