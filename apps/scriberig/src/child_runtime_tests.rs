//! Verifies bounded child runtime tools and capability delegation.

use super::inspect_catalog;
use scriberig_app::{
    Capability, CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityId,
    CapabilityScope,
};

/// Verifies project recovery tools are exposed to provider runtimes.
#[test]
fn catalog_exposes_project_recovery_operations() {
    let names = inspect_catalog()
        .definitions()
        .into_iter()
        .map(|definition| definition.name)
        .collect::<Vec<_>>();
    for name in [
        "project_import",
        "project_manifest",
        "project_repair_file",
        "project_rescan",
        "project_scan",
        "project_workspace",
    ] {
        assert!(names.iter().any(|candidate| candidate == name));
    }
    let import = inspect_catalog()
        .definitions()
        .into_iter()
        .find(|definition| definition.name == "project_import")
        .expect("project import descriptor");
    assert_eq!(
        import.parameters.0["required"],
        serde_json::json!(["project", "manifest_id"])
    );
}

/// Keeps unrestricted shell and raw-file writes out of the worker catalog.
#[test]
fn worker_catalog_does_not_grant_unrestricted_shell_or_file_writes() {
    let names = inspect_catalog()
        .definitions()
        .into_iter()
        .map(|definition| definition.name)
        .collect::<Vec<_>>();
    assert!(
        !names
            .iter()
            .any(|name| { matches!(name.as_str(), "shell" | "execute_shell" | "raw_file_write") })
    );
}

/// Exposes bounded dependency acquisition as an independently granted operation.
#[test]
fn catalog_exposes_locked_dependency_acquisition() {
    let descriptor = inspect_catalog()
        .descriptors_for(&CapabilityContext::new([CapabilityId::new(
            "project_resolve_dependencies",
        )
        .expect("dependency capability")]))
        .into_iter()
        .find(|descriptor| descriptor.definition.name == "project_resolve_dependencies")
        .expect("dependency acquisition descriptor");
    assert!(descriptor.state_changing);
    assert_eq!(
        descriptor.definition.parameters.0["required"],
        serde_json::json!(["candidate"])
    );
}

/// Advertises semantic mutation through a trusted provider-neutral category.
#[test]
fn catalog_exposes_installed_semantic_operation_entrypoint() {
    let descriptor = inspect_catalog()
        .descriptors_for(&CapabilityContext::new([CapabilityId::new(
            "domain_operation",
        )
        .expect("domain operation capability")]))
        .into_iter()
        .find(|descriptor| descriptor.definition.name == "domain_operation")
        .expect("semantic operation descriptor");

    assert!(descriptor.state_changing);
    assert_eq!(
        descriptor.category,
        scriberig_agent_runtime::tools::ToolCategory::SemanticMutation
    );
    assert_eq!(
        descriptor.definition.parameters.0["required"],
        serde_json::json!(["capability", "entity", "snapshot"])
    );
    assert_eq!(
        descriptor.definition.parameters.0["properties"]["arguments"]["additionalProperties"]["x-sensitive"],
        true
    );
    let ordered = inspect_catalog()
        .descriptors_for(&CapabilityContext::new([
            CapabilityId::new("domain_operation").expect("domain capability"),
            CapabilityId::new("project_repair_file").expect("fallback capability"),
        ]))
        .into_iter()
        .map(|descriptor| (descriptor.definition.name, descriptor.category))
        .collect::<Vec<_>>();
    let semantic_position = ordered
        .iter()
        .position(|(name, _)| name == "domain_operation")
        .expect("semantic operation position");
    let fallback_position = ordered
        .iter()
        .position(|(name, _)| name == "project_repair_file")
        .expect("fallback position");
    assert!(semantic_position < fallback_position);
}

/// Exposes spreadsheet edits as capability-gated semantic candidates.
#[test]
fn catalog_exposes_typed_spreadsheet_candidate_operation() {
    assert!(crate::agent_actions().contains(&CapabilityAction::Custom(
        "spreadsheet.set_cell_value".to_owned()
    )));
    let descriptor = inspect_catalog()
        .descriptors_for(&CapabilityContext::new([CapabilityId::new(
            "spreadsheet.set_cell_value",
        )
        .expect("spreadsheet edit capability")]))
        .into_iter()
        .find(|descriptor| descriptor.definition.name == "spreadsheet.set_cell_value")
        .expect("spreadsheet edit descriptor");

    assert!(descriptor.state_changing);
    assert_eq!(
        descriptor.category,
        scriberig_agent_runtime::tools::ToolCategory::SemanticMutation
    );
    assert_eq!(
        descriptor.definition.parameters.0["required"],
        serde_json::json!(["project", "cell", "value"])
    );
}

/// Exposes semantic document paragraph and cell edits as candidate tools.
#[test]
fn catalog_exposes_document_candidate_operations() {
    for (name, required) in [
        ("document.replace_paragraph", "paragraph"),
        ("document.update_table_cell", "cell"),
    ] {
        assert!(crate::agent_actions().contains(&CapabilityAction::Custom(name.to_owned())));
        let descriptor = inspect_catalog()
            .descriptors_for(&CapabilityContext::new([
                CapabilityId::new(name).expect("capability")
            ]))
            .into_iter()
            .find(|descriptor| descriptor.definition.name == name)
            .expect("document tool descriptor");
        assert!(descriptor.state_changing);
        assert_eq!(
            descriptor.category,
            scriberig_agent_runtime::tools::ToolCategory::SemanticMutation
        );
        assert_eq!(
            descriptor.definition.parameters.0["required"],
            serde_json::json!(["project", required, "replacement"])
        );
    }
}

/// Exposes typed CAD line and layer candidates in the integrated agent catalog.
#[test]
fn catalog_exposes_typed_cad_candidate_operations() {
    for (name, required) in [
        ("cad.update_line_geometry", "start"),
        ("cad.change_entity_layer", "destination_layer"),
    ] {
        assert!(crate::agent_actions().contains(&CapabilityAction::Custom(name.to_owned())));
        let descriptor = inspect_catalog()
            .descriptors_for(&CapabilityContext::new([
                CapabilityId::new(name).expect("CAD capability")
            ]))
            .into_iter()
            .find(|descriptor| descriptor.definition.name == name)
            .expect("CAD tool descriptor");
        assert!(descriptor.state_changing);
        assert_eq!(
            descriptor.category,
            scriberig_agent_runtime::tools::ToolCategory::SemanticMutation
        );
        assert!(
            descriptor.definition.parameters.0["required"]
                .as_array()
                .expect("required field list")
                .iter()
                .any(|field| field == required)
        );
    }
}

/// Exposes worksheet rename as an authorized semantic candidate operation.
#[test]
fn catalog_exposes_spreadsheet_sheet_rename_candidate_operation() {
    assert!(crate::agent_actions().contains(&CapabilityAction::Custom(
        "spreadsheet.rename_sheet".to_owned()
    )));
    let descriptor = inspect_catalog()
        .descriptors_for(&CapabilityContext::new([CapabilityId::new(
            "spreadsheet.rename_sheet",
        )
        .expect("spreadsheet rename capability")]))
        .into_iter()
        .find(|descriptor| descriptor.definition.name == "spreadsheet.rename_sheet")
        .expect("spreadsheet rename descriptor");

    assert!(descriptor.state_changing);
    assert_eq!(
        descriptor.category,
        scriberig_agent_runtime::tools::ToolCategory::SemanticMutation
    );
    assert_eq!(
        descriptor.definition.parameters.0["required"],
        serde_json::json!(["project", "worksheet", "new_name"])
    );
}

/// Exposes QGIS layer renames as authorized semantic candidate operations.
#[test]
fn catalog_exposes_qgis_layer_rename_candidate_operation() {
    let capability =
        CapabilityId::new("qgis.rename_layer_reference").expect("QGIS rename capability");
    assert!(crate::agent_actions().contains(&CapabilityAction::Custom(
        "qgis.rename_layer_reference".to_owned()
    )));
    let descriptor = inspect_catalog()
        .descriptors_for(&CapabilityContext::new([capability]))
        .into_iter()
        .find(|descriptor| descriptor.definition.name == "qgis.rename_layer_reference")
        .expect("QGIS rename descriptor");
    assert!(descriptor.state_changing);
    assert_eq!(
        descriptor.category,
        scriberig_agent_runtime::tools::ToolCategory::SemanticMutation
    );
    assert_eq!(
        descriptor.definition.parameters.0["required"],
        serde_json::json!(["project", "reference", "new_name"])
    );
}

/// Exposes artifact summaries as bounded reads without a mutation shortcut.
#[test]
fn catalog_exposes_read_only_bounded_artifact_inspection() {
    assert!(
        crate::agent_actions().contains(&CapabilityAction::Custom("artifact.inspect".to_owned()))
    );
    let descriptor = inspect_catalog()
        .descriptors_for(&CapabilityContext::new([CapabilityId::new(
            "artifact.inspect",
        )
        .expect("artifact inspection capability")]))
        .into_iter()
        .find(|descriptor| descriptor.definition.name == "artifact.inspect")
        .expect("artifact read descriptor");

    assert!(!descriptor.state_changing);
    assert_eq!(
        descriptor.category,
        scriberig_agent_runtime::tools::ToolCategory::SemanticRead
    );
    assert_eq!(
        descriptor.definition.parameters.0["required"],
        serde_json::json!(["project"])
    );
    assert_eq!(
        descriptor.required_capability.as_deref(),
        Some("artifact.inspect")
    );
}

/// Exposes geospatial attribute edits as capability-gated semantic candidates.
#[test]
fn catalog_exposes_typed_geo_attribute_candidate_operation() {
    assert!(
        !crate::agent_actions().contains(&CapabilityAction::Custom("geo.set_attribute".to_owned()))
    );
    let descriptor = inspect_catalog()
        .descriptors_for(&CapabilityContext::new([CapabilityId::new(
            "geo.set_attribute",
        )
        .expect("geo edit capability")]))
        .into_iter()
        .find(|descriptor| descriptor.definition.name == "geo.set_attribute")
        .expect("geo attribute edit descriptor");

    assert!(descriptor.state_changing);
    assert_eq!(
        descriptor.category,
        scriberig_agent_runtime::tools::ToolCategory::SemanticMutation
    );
    assert_eq!(
        descriptor.definition.parameters.0["required"],
        serde_json::json!(["project", "path", "layer", "feature", "field", "value"])
    );
}

/// Exposes exact-snapshot geospatial geometry replacement to authorized branches.
#[test]
fn catalog_exposes_typed_geo_geometry_candidate_operation() {
    assert!(
        !crate::agent_actions()
            .contains(&CapabilityAction::Custom("geo.replace_geometry".to_owned()))
    );
    let descriptor = inspect_catalog()
        .descriptors_for(&CapabilityContext::new([CapabilityId::new(
            "geo.replace_geometry",
        )
        .expect("geo geometry capability")]))
        .into_iter()
        .find(|descriptor| descriptor.definition.name == "geo.replace_geometry")
        .expect("geo geometry edit descriptor");

    assert!(descriptor.state_changing);
    assert_eq!(
        descriptor.category,
        scriberig_agent_runtime::tools::ToolCategory::SemanticMutation
    );
    assert_eq!(
        descriptor.definition.parameters.0["required"],
        serde_json::json!(["project", "path", "layer", "feature", "geometry"])
    );
}

#[path = "child_runtime_tests/delegated_capabilities.rs"]
mod delegated_capabilities;
