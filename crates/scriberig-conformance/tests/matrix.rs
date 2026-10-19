//! Validates the repository capability matrix vocabulary and completeness.

use std::collections::BTreeSet;

const SURFACES: [&str; 9] = [
    "library", "app", "protocol", "server", "client", "cli", "tui", "web", "agent",
];
const STATUSES: [&str; 4] = ["not_applicable", "planned", "implemented", "verified"];

/// Parses and validates one capability matrix document.
///
/// - `source`: TOML capability matrix.
///
/// Success when all entries use the prescribed vocabulary.
///
/// Returns a readable validation error for malformed entries.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn validate_matrix(source: &str) -> Result<(), String> {
    let document: toml::Value = source.parse().map_err(|error| format!("parse: {error}"))?;
    let entries = document
        .get("capabilities")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| "capabilities must be an array".to_owned())?;
    let mut ids = BTreeSet::new();
    for entry in entries {
        let table = entry
            .as_table()
            .ok_or_else(|| "capability entry must be a table".to_owned())?;
        let id = table
            .get("id")
            .and_then(toml::Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "capability id is required".to_owned())?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("duplicate capability id: {id}"));
        }
        if table
            .get("description")
            .and_then(toml::Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(format!("empty description: {id}"));
        }
        if table
            .get("semantic_owner")
            .and_then(toml::Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(format!("missing semantic owner: {id}"));
        }
        let surfaces = table
            .get("surfaces")
            .and_then(toml::Value::as_table)
            .ok_or_else(|| format!("missing surfaces: {id}"))?;
        if surfaces.len() != SURFACES.len() {
            return Err(format!("wrong surface count: {id}"));
        }
        for surface in SURFACES {
            let status = surfaces
                .get(surface)
                .and_then(toml::Value::as_str)
                .ok_or_else(|| format!("missing surface {surface}: {id}"))?;
            if !STATUSES.contains(&status) {
                return Err(format!("unknown status {status}: {id}"));
            }
        }
    }
    Ok(())
}

/// Verifies the checked-in matrix has exactly the declared capability entries.
#[test]
fn checked_in_matrix_is_truthful_and_complete() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../capabilities.toml");
    let source = std::fs::read_to_string(path).expect("capabilities.toml");
    validate_matrix(&source).expect("valid matrix");
    let document: toml::Value = source.parse().expect("toml");
    let ids: Vec<_> = document["capabilities"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|entry| entry["id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        ids,
        vec![
            "inspect_entity",
            "runtime_capabilities",
            "operation_status",
            "operation_cancel",
            "event_subscription",
            "agent_start",
            "agent_continue",
            "agent_final_result",
            "session_branch_tree",
            "session_branch_activity",
            "agent_event_subscription",
            "agent_streaming_output",
            "session_model_selection",
            "agent_presentation_preferences",
            "model_request_inspection",
            "project_workspace",
            "semantic_code_navigation",
            "source_projection",
            "projection_edit_candidate",
            "filesystem_materialize",
            "semantic_project_agent_candidate",
        ]
    );
}

/// Verifies TUI statuses match the currently verified implementation slices.
#[test]
fn tui_statuses_match_implemented_surface_evidence() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../capabilities.toml");
    let source = std::fs::read_to_string(path).expect("capabilities.toml");
    let document: toml::Value = source.parse().expect("toml");
    let expected = [
        ("inspect_entity", "verified"),
        ("runtime_capabilities", "implemented"),
        ("operation_status", "implemented"),
        ("operation_cancel", "implemented"),
        ("event_subscription", "implemented"),
        ("agent_start", "implemented"),
        ("agent_continue", "implemented"),
        ("agent_final_result", "implemented"),
        ("session_branch_tree", "implemented"),
        ("session_branch_activity", "implemented"),
        ("agent_event_subscription", "implemented"),
        ("agent_streaming_output", "implemented"),
        ("project_workspace", "verified"),
        ("semantic_code_navigation", "verified"),
        ("source_projection", "verified"),
        ("projection_edit_candidate", "verified"),
        ("filesystem_materialize", "verified"),
        ("semantic_project_agent_candidate", "verified"),
    ];
    for (id, status) in expected {
        let entry = document["capabilities"]
            .as_array()
            .expect("entries")
            .iter()
            .find(|entry| entry["id"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("missing capability {id}"));
        assert_eq!(entry["surfaces"]["tui"].as_str(), Some(status), "{id}");
    }
}

/// Verifies malformed matrix entries are rejected with readable diagnostics.
#[test]
fn malformed_matrix_is_rejected() {
    let duplicate = "[[capabilities]]\nid = \"same\"\ndescription = \"x\"\nsemantic_owner = \"owner\"\n[capabilities.surfaces]\nlibrary = \"planned\"\n";
    assert!(validate_matrix(duplicate).is_err());
}
