//! Decodes stable HTTP operation names into typed protocol requests.

use serde_json::{Value, json};

#[allow(
    clippy::too_many_lines,
    reason = "keeping the stable HTTP operation mapping in one audited match"
)]
/// Decodes one stable operation identifier into a protocol request variant.
///
/// - `operation`: Stable operation identifier from the URL.
/// - `body`: JSON request payload.
///
/// The corresponding typed protocol request.
///
/// Returns a transport status and structured body when decoding fails.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn decode_request(
    operation: &str,
    body: &str,
) -> Result<scriberig_protocol::Request, (u16, Value)> {
    let invalid = |error: serde_json::Error| {
        (
            400,
            json!({"code": "invalid_request", "message": error.to_string()}),
        )
    };
    let request = match operation {
        "cad_drawing_inspect" => scriberig_protocol::Request::CadDrawingInspect(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "cad_entities" => {
            scriberig_protocol::Request::CadEntities(serde_json::from_str(body).map_err(invalid)?)
        }
        "cad_text_find" => {
            scriberig_protocol::Request::CadTextFind(serde_json::from_str(body).map_err(invalid)?)
        }
        "cad_bounds_query" => scriberig_protocol::Request::CadBoundsQuery(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "cad_line_edit" => {
            scriberig_protocol::Request::CadLineEdit(serde_json::from_str(body).map_err(invalid)?)
        }
        "cad_layer_change" => scriberig_protocol::Request::CadLayerChange(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "artifact_inspect" => scriberig_protocol::Request::ArtifactInspect(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "artifact_members" => scriberig_protocol::Request::ArtifactMembers(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "spreadsheet_cells" => scriberig_protocol::Request::SpreadsheetCells(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "spreadsheet_cell_inspect" => scriberig_protocol::Request::SpreadsheetCellInspect(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "spreadsheet_workbook" => scriberig_protocol::Request::SpreadsheetWorkbook(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "spreadsheet_edit" => scriberig_protocol::Request::SpreadsheetEdit(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "spreadsheet_range_edit" => scriberig_protocol::Request::SpreadsheetRangeEdit(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "spreadsheet_rename" => scriberig_protocol::Request::SpreadsheetRename(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "document_paragraph_edit" => scriberig_protocol::Request::DocumentParagraphEdit(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "document_table_cell_edit" => scriberig_protocol::Request::DocumentTableCellEdit(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "document_paragraph_insert" => scriberig_protocol::Request::DocumentParagraphInsert(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "document_inspect" => scriberig_protocol::Request::DocumentInspect(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "document_paragraph_find" => scriberig_protocol::Request::DocumentParagraphFind(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "spreadsheet_cell_find" => scriberig_protocol::Request::SpreadsheetCellFind(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "geo_features" => {
            scriberig_protocol::Request::GeoFeatures(serde_json::from_str(body).map_err(invalid)?)
        }
        "geo_feature_query" => scriberig_protocol::Request::GeoFeatureQuery(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "geo_catalog" => {
            scriberig_protocol::Request::GeoCatalog(serde_json::from_str(body).map_err(invalid)?)
        }
        "qgis_project" => {
            scriberig_protocol::Request::QgisProject(serde_json::from_str(body).map_err(invalid)?)
        }
        "qgis_layer_rename" => scriberig_protocol::Request::QgisLayerRename(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "qgis_layer_order" => scriberig_protocol::Request::QgisLayerOrder(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "geo_geometry_edit" => scriberig_protocol::Request::GeoGeometryEdit(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "geo_attribute_edit" => scriberig_protocol::Request::GeoAttributeEdit(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "inspect" => {
            scriberig_protocol::Request::Inspect(serde_json::from_str(body).map_err(invalid)?)
        }
        "capabilities" => {
            scriberig_protocol::Request::Capabilities(serde_json::from_str(body).map_err(invalid)?)
        }
        "operation" | "operation_status" => scriberig_protocol::Request::OperationStatus(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "cancel" | "cancel_operation" => scriberig_protocol::Request::CancelOperation(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "subscribe" => {
            scriberig_protocol::Request::Subscribe(serde_json::from_str(body).map_err(invalid)?)
        }
        "query" => scriberig_protocol::Request::Query(serde_json::from_str(body).map_err(invalid)?),
        "list" => scriberig_protocol::Request::List(serde_json::from_str(body).map_err(invalid)?),
        "find" => scriberig_protocol::Request::Find(serde_json::from_str(body).map_err(invalid)?),
        "command" => {
            scriberig_protocol::Request::Command(serde_json::from_str(body).map_err(invalid)?)
        }
        "agent_start" => {
            scriberig_protocol::Request::AgentStart(serde_json::from_str(body).map_err(invalid)?)
        }
        "agent_new_session" => scriberig_protocol::Request::AgentNewSession(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "agent_continue" => {
            scriberig_protocol::Request::AgentContinue(serde_json::from_str(body).map_err(invalid)?)
        }
        "agent_continue_at" => scriberig_protocol::Request::AgentContinueAt(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "session" | "session_read" => {
            scriberig_protocol::Request::SessionRead(serde_json::from_str(body).map_err(invalid)?)
        }
        "plan_create" => {
            scriberig_protocol::Request::PlanCreate(serde_json::from_str(body).map_err(invalid)?)
        }
        "plan_lifecycle" => {
            scriberig_protocol::Request::PlanLifecycle(serde_json::from_str(body).map_err(invalid)?)
        }
        "plan_assignment" => scriberig_protocol::Request::PlanAssignment(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "plan_replan" => {
            scriberig_protocol::Request::PlanReplan(serde_json::from_str(body).map_err(invalid)?)
        }
        "plan_mutation" => {
            scriberig_protocol::Request::PlanMutation(serde_json::from_str(body).map_err(invalid)?)
        }
        "plan_attempts" => {
            scriberig_protocol::Request::PlanAttempts(serde_json::from_str(body).map_err(invalid)?)
        }
        "plan_details" => {
            scriberig_protocol::Request::PlanDetails(serde_json::from_str(body).map_err(invalid)?)
        }
        "plan_view" => {
            scriberig_protocol::Request::PlanView(serde_json::from_str(body).map_err(invalid)?)
        }
        "model_request_detail" => scriberig_protocol::Request::ModelRequestDetail(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "tool_result_detail" => scriberig_protocol::Request::ToolResultDetail(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "model_library" => {
            scriberig_protocol::Request::ModelLibrary(serde_json::from_str(body).map_err(invalid)?)
        }
        "agent_presentation_preferences" => {
            scriberig_protocol::Request::AgentPresentationPreferencesRead(
                serde_json::from_str(body).map_err(invalid)?,
            )
        }
        "set_agent_presentation_preferences" => {
            scriberig_protocol::Request::AgentPresentationPreferencesSet(
                serde_json::from_str(body).map_err(invalid)?,
            )
        }
        "session_model_selection" => scriberig_protocol::Request::SessionModelSelection(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "session_thinking_selection" => scriberig_protocol::Request::SessionThinkingSelection(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "legacy_execution_records" => scriberig_protocol::Request::LegacyExecutionRecords(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "legacy_execution_mapping" => scriberig_protocol::Request::LegacyExecutionMapping(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "legacy_plan_assignment_mapping" => {
            scriberig_protocol::Request::LegacyPlanAssignmentMapping(
                serde_json::from_str(body).map_err(invalid)?,
            )
        }
        "projection_style_schema" => scriberig_protocol::Request::ProjectionStyleSchema(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "projection_style_resolve" => scriberig_protocol::Request::ProjectionStyleResolve(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "projection_style_setting" => scriberig_protocol::Request::ProjectionStyleSetting(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "project" => {
            scriberig_protocol::Request::Project(serde_json::from_str(body).map_err(invalid)?)
        }
        _ => {
            return Err((
                404,
                json!({
                    "code": "unsupported_operation",
                    "message": format!("unknown protocol operation: {operation}"),
                }),
            ));
        }
    };
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::decode_request;

    /// Verifies HTTP operations decode model-library and selection requests.
    #[test]
    fn model_library_and_selection_operations_decode() {
        assert!(matches!(
            decode_request("model_library", "{}"),
            Ok(scriberig_protocol::Request::ModelLibrary(_))
        ));
        assert!(matches!(
            decode_request(
                "model_request_detail",
                r#"{"session":{"id":"session-1"},"branch":{"id":"branch-1"},"invocation":"invocation-1"}"#
            ),
            Ok(scriberig_protocol::Request::ModelRequestDetail(_))
        ));
        assert!(matches!(
            decode_request(
                "tool_result_detail",
                r#"{"session":{"id":"session-1"},"branch":{"id":"branch-1"},"result":"result-1"}"#
            ),
            Ok(scriberig_protocol::Request::ToolResultDetail(_))
        ));
        assert!(matches!(
            decode_request(
                "session_model_selection",
                r#"{"session":{"id":"session-1"},"expected_revision":2,"provider_id":"provider-1","model_id":"model-1"}"#
            ),
            Ok(scriberig_protocol::Request::SessionModelSelection(_))
        ));
        assert!(matches!(
            decode_request(
                "session_thinking_selection",
                r#"{"session":{"id":"session-1"},"expected_revision":3,"thinking_level":"deep-dive"}"#
            ),
            Ok(scriberig_protocol::Request::SessionThinkingSelection(_))
        ));
        assert!(matches!(
            decode_request(
                "geo_geometry_edit",
                r#"{"project":"project-1","path":"parcels.gpkg","snapshot":"snapshot-1","layer":"layer-1","feature":"feature-1","geometry":{"kind":"point","coordinates":{"x":"1","y":"2","z":null,"measure":null}}}"#
            ),
            Ok(scriberig_protocol::Request::GeoGeometryEdit(_))
        ));
        assert!(matches!(
            decode_request(
                "geo_attribute_edit",
                r#"{"project":"project-1","path":"parcels.gpkg","snapshot":"snapshot-1","layer":"layer-1","feature":"feature-1","field":"area","value":{"kind":"number","value":"42.5"}}"#
            ),
            Ok(scriberig_protocol::Request::GeoAttributeEdit(_))
        ));
        assert!(matches!(
            decode_request(
                "legacy_execution_records",
                r#"{"session":{"id":"session-1"}}"#
            ),
            Ok(scriberig_protocol::Request::LegacyExecutionRecords(_))
        ));
        assert!(matches!(
            decode_request(
                "legacy_execution_mapping",
                r#"{"session":{"id":"session-1"},"expected_revision":3,"legacy_path":"/execution","provider_id":"provider-1","model_id":"model-1"}"#
            ),
            Ok(scriberig_protocol::Request::LegacyExecutionMapping(_))
        ));
        assert!(matches!(
            decode_request(
                "legacy_plan_assignment_mapping",
                r#"{"session":{"id":"session-1"},"legacy_path":"/plans/0/nodes/0/assignment","expected_plan_revision":4,"provider_id":"provider-1","model_id":"model-1"}"#
            ),
            Ok(scriberig_protocol::Request::LegacyPlanAssignmentMapping(_))
        ));
        assert!(matches!(
            decode_request(
                "plan_attempts",
                r#"{"session":{"id":"session-1"},"plan":"plan-1","node":"node-1","offset":5,"limit":10}"#
            ),
            Ok(scriberig_protocol::Request::PlanAttempts(_))
        ));
        assert!(matches!(
            decode_request(
                "plan_details",
                r#"{"session":{"id":"session-1"},"plan":"plan-1","expected_revision":9,"gate_offset":0,"evidence_offset":10,"limit":25}"#
            ),
            Ok(scriberig_protocol::Request::PlanDetails(_))
        ));
    }

    /// Verifies HTTP style operations decode their purpose and scope enums.
    #[test]
    fn projection_style_operations_decode() {
        assert!(matches!(
            decode_request(
                "projection_style_schema",
                r#"{"projection_kind":"javascript"}"#
            ),
            Ok(scriberig_protocol::Request::ProjectionStyleSchema(_))
        ));
        assert!(matches!(
            decode_request(
                "projection_style_resolve",
                r#"{"projection_kind":"javascript","project_id":"p","workspace_id":"w","purpose":"InteractiveView","view_id":"view-1","explicit_personal":null}"#
            ),
            Ok(scriberig_protocol::Request::ProjectionStyleResolve(_))
        ));
        assert!(matches!(
            decode_request(
                "projection_style_setting",
                r#"{"scope":"User","projection_kind":"javascript","expected_revision":null,"style":[113,117,111,116,101,61,115,105,110,103,108,101]}"#
            ),
            Ok(scriberig_protocol::Request::ProjectionStyleSetting(_))
        ));
    }

    /// Verifies the typed CAD layer operation has an HTTP request route.
    #[test]
    fn cad_layer_change_operation_decodes() {
        assert!(matches!(
            decode_request(
                "cad_layer_change",
                r#"{"project":"project-1","snapshot":"snapshot-2","entity":"entity-3","destination_layer":"layer-4"}"#
            ),
            Ok(scriberig_protocol::Request::CadLayerChange(_))
        ));
    }

    /// Verifies CAD bounds queries have a stable HTTP route and typed decoding.
    #[test]
    fn cad_bounds_query_operation_decodes() {
        assert!(matches!(
            decode_request(
                "cad_bounds_query",
                r#"{"project":"project-1","snapshot":"snapshot-2","drawing":"entity-3","space":{"Model":null},"bounds":{"min_x":"0","min_y":"0","max_x":"1","max_y":"1"},"offset":0,"limit":10}"#
            ),
            Ok(scriberig_protocol::Request::CadBoundsQuery(_))
        ));
    }

    /// Verifies bounded CAD text search has a stable HTTP route and typed decoding.
    #[test]
    fn cad_text_find_operation_decodes() {
        assert!(matches!(
            decode_request(
                "cad_text_find",
                r#"{"project":"project-1","snapshot":"snapshot-2","drawing":"drawing-3","query":"parcel","offset":0,"limit":10}"#
            ),
            Ok(scriberig_protocol::Request::CadTextFind(_))
        ));
    }
}
