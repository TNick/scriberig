//! Defines the trusted provider-visible tool catalog.

use super::control_parameters::control_parameters;
use rpi_ai::types::Tool;
use scriberig_agent_runtime::tools::ToolCategory;

/// Builds the trusted provider-visible catalog for the local agent runtime.
///
/// Deterministically ordered inspect, control, and delegation descriptors.
///
/// - `super`: Module namespace containing the catalog composition helper.
#[allow(clippy::too_many_lines)]
/// # Returns
/// The operation result.
pub(crate) fn inspect_catalog() -> scriberig_agent_runtime::tools::ToolCatalog {
    use scriberig_agent_runtime::tools::{
        ToolCatalog, ToolCategory, ToolDescriptor, ToolExecutionMode,
    };

    let mut descriptors = vec![
        ToolDescriptor {
            definition: Tool {
                name: "inspect".to_owned(),
                description: "Inspect one semantic entity at an exact snapshot.".to_owned(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "entity": {"type": "string"},
                        "snapshot": {"type": "string"},
                        "detail": {"type": "string", "enum": ["compact", "detailed"]}
                    },
                    "required": ["entity"]
                })
                .into(),
                constrained_sampling: None,
            },
            category: ToolCategory::SemanticRead,
            result_shape: "compact entity handles and resolved revision".to_owned(),
            invocation_label: Some("Inspect entity".to_owned()),
            state_changing: false,
            execution_mode: ToolExecutionMode::Atomic,
            required_capability: Some("inspect.entity".to_owned()),
        },
        ToolDescriptor {
            definition: Tool {
                name: "delegate".to_owned(),
                description: "Delegate focused reasoning to an isolated child branch.".to_owned(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "purpose": {"type": "string"},
                        "prompt": {"type": "string"},
                        "capabilities": {"type": "array", "items": {"type": "string"}},
                        "expected_result": {"type": "string"},
                        "execution": {
                            "type": "object",
                            "properties": {
                                "provider_id": {"type": "string"},
                                "model_id": {"type": "string"},
                                "thinking_level": {"type": ["string", "null"]}
                            },
                            "required": ["provider_id", "model_id"]
                        }
                    },
                    "required": ["purpose", "prompt", "capabilities", "expected_result"]
                })
                .into(),
                constrained_sampling: None,
            },
            category: ToolCategory::BranchProducing,
            result_shape: "compact child branch result and handles".to_owned(),
            invocation_label: None,
            state_changing: false,
            execution_mode: ToolExecutionMode::ChildBranch,
            required_capability: Some("agent.delegate".to_owned()),
        },
    ];
    for (name, description, required_capability) in [
        (
            "runtime_capabilities",
            "Discover exposed runtime capabilities.",
            "runtime_capabilities",
        ),
        (
            "operation_status",
            "Read one operation lifecycle status.",
            "operation_status",
        ),
        (
            "operation_cancel",
            "Cancel one active operation.",
            "operation_cancel",
        ),
        (
            "event_subscription",
            "Read ordered lifecycle events.",
            "event_subscription",
        ),
        (
            "agent_start",
            "Accept agent work for a session.",
            "agent_start",
        ),
        (
            "agent_continue",
            "Continue work on a session branch.",
            "agent_continue",
        ),
        (
            "session_branch_tree",
            "Read a durable session branch tree.",
            "session_branch_tree",
        ),
        (
            "session_branch_activity",
            "Read ordered branch activity.",
            "session_branch_activity",
        ),
        (
            "agent_event_subscription",
            "Read ordered agent lifecycle events.",
            "agent_event_subscription",
        ),
        (
            "agent_streaming_output",
            "Read bounded agent output events.",
            "agent_streaming_output",
        ),
        (
            "project_scan",
            "Scan the trusted current working directory into a persisted project manifest.",
            "project_scan",
        ),
        (
            "project_manifest",
            "Review a bounded page of an exact persisted project scan manifest.",
            "project_manifest",
        ),
        (
            "project_import",
            "Import the exact reviewed scan manifest and return semantic workspace references.",
            "project_import",
        ),
        (
            "project_workspace",
            "Resolve the current exact semantic workspace for an imported project.",
            "project_workspace",
        ),
        (
            "semantic_find",
            "Find project-owned source and declaration references in one exact snapshot.",
            "semantic_find",
        ),
        (
            "semantic_render",
            "Render one project source through an exact semantic entity reference.",
            "semantic_render",
        ),
        (
            "domain_operation",
            "Execute one installed semantic operation against an exact entity and snapshot.",
            "domain_operation",
        ),
        (
            "cad.update_line_geometry",
            "Propose exact start and end coordinates for one selected CAD line.",
            "cad.update_line_geometry",
        ),
        (
            "cad.change_entity_layer",
            "Propose moving one selected CAD entity to an existing drawing layer.",
            "cad.change_entity_layer",
        ),
        (
            "spreadsheet.set_cell_value",
            "Propose one typed cell value or formula change against an exact workbook snapshot.",
            "spreadsheet.set_cell_value",
        ),
        (
            "document.replace_paragraph",
            "Propose replacement text for one semantic document paragraph.",
            "document.replace_paragraph",
        ),
        (
            "document.update_table_cell",
            "Propose replacement text for one semantic document table cell.",
            "document.update_table_cell",
        ),
        (
            "spreadsheet.read_range",
            "Read a bounded sparse cell range from an exact workbook snapshot.",
            "spreadsheet.read_range",
        ),
        (
            "spreadsheet.rename_sheet",
            "Propose a worksheet rename with supported dependent references updated.",
            "spreadsheet.rename_sheet",
        ),
        (
            "artifact.inspect",
            "Read a bounded page of artifact member and fidelity summaries without source bytes.",
            "artifact.inspect",
        ),
        (
            "geo.set_attribute",
            "Propose one typed feature attribute change against an exact geospatial snapshot.",
            "geo.set_attribute",
        ),
        (
            "geo.replace_geometry",
            "Propose one typed feature geometry change against an exact geospatial snapshot.",
            "geo.replace_geometry",
        ),
        (
            "qgis.rename_layer_reference",
            "Propose a display-name change for one exact QGIS layer reference.",
            "qgis.rename_layer_reference",
        ),
        (
            "project_rescan",
            "Rescan a project after a source repair.",
            "project_rescan",
        ),
        (
            "project_repair_file",
            "Repair one authorized project-relative source file.",
            "project_repair_file",
        ),
        (
            "project_propose",
            "Propose an exact-base multi-file source candidate through the fallback projection path.",
            "project_propose",
        ),
        (
            "project_merge_candidates",
            "Integrate two exact-base dependency candidates into a new reviewable candidate.",
            "project_merge_candidates",
        ),
        (
            "project_candidate",
            "Inspect a durable project-wide candidate and its path effects.",
            "project_candidate",
        ),
        (
            "project_validate_candidate",
            "Validate one exact project candidate in an isolated workspace.",
            "project_validate_candidate",
        ),
        (
            "project_resolve_dependencies",
            "Acquire dependencies pinned by an exact candidate lockfile with bounded network access.",
            "project_resolve_dependencies",
        ),
    ] {
        descriptors.push(ToolDescriptor {
            definition: Tool {
                name: name.to_owned(),
                description: description.to_owned(),
                parameters: control_parameters(name).into(),
                constrained_sampling: None,
            },
            category: tool_category(name),
            result_shape: "bounded provider-neutral control result".to_owned(),
            invocation_label: None,
            state_changing: matches!(
                name,
                "operation_cancel"
                    | "agent_start"
                    | "agent_continue"
                    | "project_scan"
                    | "project_import"
                    | "project_rescan"
                    | "project_repair_file"
                    | "project_propose"
                    | "project_merge_candidates"
                    | "project_validate_candidate"
                    | "project_resolve_dependencies"
                    | "domain_operation"
                    | "cad.update_line_geometry"
                    | "cad.change_entity_layer"
                    | "spreadsheet.set_cell_value"
                    | "document.replace_paragraph"
                    | "document.update_table_cell"
                    | "spreadsheet.rename_sheet"
                    | "geo.set_attribute"
                    | "geo.replace_geometry"
                    | "qgis.rename_layer_reference"
            ),
            execution_mode: ToolExecutionMode::Atomic,
            required_capability: Some(required_capability.to_owned()),
        });
    }
    ToolCatalog::new(descriptors)
}

/// Classifies built-in control tools using trusted registration metadata.
/// # Parameters
/// - `name`: Input for this operation.
/// # Returns
/// The operation result.
fn tool_category(name: &str) -> ToolCategory {
    match name {
        "domain_operation"
        | "cad.update_line_geometry"
        | "cad.change_entity_layer"
        | "spreadsheet.set_cell_value"
        | "document.replace_paragraph"
        | "document.update_table_cell"
        | "spreadsheet.rename_sheet"
        | "geo.set_attribute"
        | "geo.replace_geometry"
        | "qgis.rename_layer_reference"
        | "project_merge_candidates" => ToolCategory::SemanticMutation,
        "project_propose" | "project_repair_file" => ToolCategory::OpaqueFallback,
        "project_validate_candidate" | "project_resolve_dependencies" => {
            ToolCategory::ValidationProcess
        }
        "agent_start" | "agent_continue" => ToolCategory::BranchProducing,
        "project_scan" | "project_import" | "project_rescan" => ToolCategory::ProjectionOperation,
        _ => ToolCategory::SemanticRead,
    }
}
