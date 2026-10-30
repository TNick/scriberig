//! Defines bounded JSON schemas for local control tools.

/// Defines precise JSON inputs for project bootstrap tools.
///
/// - `name`: Registered control operation.
/// - `super`: Parent module containing the child runtime catalog.
///
/// The provider-visible JSON Schema for the operation's bounded arguments.
/// # Returns
/// The operation result.
#[allow(clippy::too_many_lines)] // The accepted parameter schema is intentionally explicit.
pub(super) fn control_parameters(name: &str) -> serde_json::Value {
    match name {
        "project_scan" => serde_json::json!({
            "type": "object",
            "properties": {
                "path_ref": {"type": "string", "enum": ["current_working_directory"]},
                "name": {"type": "string"},
                "git_tracked": {"type": "boolean"}
            }
        }),
        "project_manifest" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "manifest_id": {"type": "string", "minLength": 64, "maxLength": 64},
                "offset": {"type": "integer", "minimum": 0},
                "limit": {"type": "integer", "minimum": 1, "maximum": 100}
            },
            "required": ["project", "manifest_id"]
        }),
        "project_import" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "manifest_id": {"type": "string", "minLength": 64, "maxLength": 64}
            },
            "required": ["project", "manifest_id"]
        }),
        "project_workspace" => serde_json::json!({
            "type": "object",
            "properties": {"project": {"type": "string"}},
            "required": ["project"]
        }),
        "semantic_find" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "snapshot": {"type": "string"},
                "query": {"type": "string", "minLength": 1},
                "limit": {"type": "integer", "minimum": 1, "maximum": 50}
            },
            "required": ["project", "snapshot", "query"]
        }),
        "semantic_render" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "snapshot": {"type": "string"},
                "entity": {"type": "string"}
            },
            "required": ["project", "snapshot", "entity"]
        }),
        "domain_operation" => serde_json::json!({
            "type": "object",
            "properties": {
                "capability": {"type": "string"},
                "entity": {"type": "string"},
                "snapshot": {"type": "string"},
                "revision": {"type": "string"},
                "branch": {"type": "string"},
                "arguments": {"type": "object", "additionalProperties": {
                    "type": ["string", "integer", "boolean"],
                    "x-sensitive": true
                }}
            },
            "required": ["capability", "entity", "snapshot"]
        }),
        "cad.update_line_geometry" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "entity": {"type": "string"},
                "start": {"$ref": "#/$defs/coordinate"},
                "end": {"$ref": "#/$defs/coordinate"}
            },
            "$defs": {
                "coordinate": {
                    "type": "object",
                    "properties": {"x": {"type": "string"}, "y": {"type": "string"}, "z": {"type": "string"}},
                    "required": ["x", "y", "z"],
                    "additionalProperties": false
                }
            },
            "required": ["project", "entity", "start", "end"],
            "additionalProperties": false
        }),
        "cad.change_entity_layer" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "entity": {"type": "string"},
                "destination_layer": {"type": "string"}
            },
            "required": ["project", "entity", "destination_layer"],
            "additionalProperties": false
        }),
        "spreadsheet.set_cell_value" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "cell": {"type": "string"},
                "value": {
                    "oneOf": [
                        {"type": "object", "properties": {"kind": {"const": "blank"}}, "required": ["kind"], "additionalProperties": false},
                        {"type": "object", "properties": {"kind": {"const": "text"}, "value": {"type": "string"}}, "required": ["kind", "value"], "additionalProperties": false},
                        {"type": "object", "properties": {"kind": {"const": "number"}, "value": {"type": "string"}}, "required": ["kind", "value"], "additionalProperties": false},
                        {"type": "object", "properties": {"kind": {"const": "boolean"}, "value": {"type": "boolean"}}, "required": ["kind", "value"], "additionalProperties": false},
                        {"type": "object", "properties": {"kind": {"const": "error"}, "value": {"type": "string"}}, "required": ["kind", "value"], "additionalProperties": false},
                        {"type": "object", "properties": {"kind": {"const": "formula"}, "value": {"type": "object", "properties": {"expression": {"type": "string"}, "cachedValue": {"type": ["object", "null"]}}, "required": ["expression"]}}, "required": ["kind", "value"], "additionalProperties": false}
                    ]
                }
            },
            "required": ["project", "cell", "value"]
        }),
        "document.replace_paragraph" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "paragraph": {"type": "string"},
                "replacement": {"type": "string", "minLength": 1}
            },
            "required": ["project", "paragraph", "replacement"],
            "additionalProperties": false
        }),
        "document.update_table_cell" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "cell": {"type": "string"},
                "replacement": {"type": "string", "minLength": 1}
            },
            "required": ["project", "cell", "replacement"],
            "additionalProperties": false
        }),
        "spreadsheet.read_range" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "worksheet": {"type": "string"},
                "range": {
                    "type": ["object", "null"],
                    "properties": {
                        "first_row": {"type": "integer", "minimum": 1},
                        "first_column": {"type": "integer", "minimum": 1},
                        "last_row": {"type": "integer", "minimum": 1},
                        "last_column": {"type": "integer", "minimum": 1}
                    },
                    "required": ["first_row", "first_column", "last_row", "last_column"],
                    "additionalProperties": false
                },
                "offset": {"type": "integer", "minimum": 0},
                "limit": {"type": "integer", "minimum": 1, "maximum": 500}
            },
            "required": ["project", "worksheet"],
            "additionalProperties": false
        }),
        "spreadsheet.rename_sheet" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "worksheet": {"type": "string"},
                "new_name": {"type": "string", "minLength": 1, "maxLength": 31}
            },
            "required": ["project", "worksheet", "new_name"],
            "additionalProperties": false
        }),
        "artifact.inspect" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string", "minLength": 1},
                "offset": {"type": "integer", "minimum": 0},
                "limit": {"type": "integer", "minimum": 1, "maximum": 100}
            },
            "required": ["project"],
            "additionalProperties": false
        }),
        "geo.set_attribute" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "path": {"type": "string", "minLength": 1},
                "layer": {"type": "string"},
                "feature": {"type": "string"},
                "field": {"type": "string", "minLength": 1},
                "value": {
                    "oneOf": [
                        {"type": "string", "const": "Null"},
                        {"type": "object", "properties": {"Text": {"type": "string"}}, "required": ["Text"], "additionalProperties": false},
                        {"type": "object", "properties": {"Number": {"type": "string"}}, "required": ["Number"], "additionalProperties": false},
                        {"type": "object", "properties": {"Boolean": {"type": "boolean"}}, "required": ["Boolean"], "additionalProperties": false},
                        {"type": "object", "properties": {"DateTime": {"type": "string"}}, "required": ["DateTime"], "additionalProperties": false}
                    ]
                }
            },
            "required": ["project", "path", "layer", "feature", "field", "value"],
            "additionalProperties": false
        }),
        "geo.replace_geometry" => serde_json::json!({
            "$defs": {
                "coordinate": {
                    "type": "object",
                    "properties": {
                        "x": {"type": "string"}, "y": {"type": "string"},
                        "z": {"type": ["string", "null"]},
                        "measure": {"type": ["string", "null"]}
                    },
                    "required": ["x", "y"], "additionalProperties": false
                },
                "geometry": {
                    "oneOf": [
                        {"type": "object", "properties": {"Point": {"$ref": "#/$defs/coordinate"}}, "required": ["Point"], "additionalProperties": false},
                        {"type": "object", "properties": {"LineString": {"type": "array", "items": {"$ref": "#/$defs/coordinate"}, "minItems": 2}}, "required": ["LineString"], "additionalProperties": false},
                        {"type": "object", "properties": {"Polygon": {"type": "array", "items": {"type": "array", "items": {"$ref": "#/$defs/coordinate"}, "minItems": 4}, "minItems": 1}}, "required": ["Polygon"], "additionalProperties": false},
                        {"type": "object", "properties": {"Collection": {"type": "array", "items": {"$ref": "#/$defs/geometry"}}}, "required": ["Collection"], "additionalProperties": false}
                    ]
                }
            },
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "path": {"type": "string", "minLength": 1},
                "layer": {"type": "string"},
                "feature": {"type": "string"},
                "geometry": {"$ref": "#/$defs/geometry"}
            },
            "required": ["project", "path", "layer", "feature", "geometry"],
            "additionalProperties": false
        }),
        "qgis.rename_layer_reference" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "reference": {"type": "string"},
                "new_name": {"type": "string", "minLength": 1, "maxLength": 1024}
            },
            "required": ["project", "reference", "new_name"],
            "additionalProperties": false
        }),
        "project_propose" => serde_json::json!({
            "type": "object",
            "properties": {
                "project": {"type": "string"},
                "base_snapshot": {"type": "string"},
                "effects": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 512,
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"},
                            "kind": {"type": "string", "enum": ["create", "update", "delete"]},
                            "content": {"oneOf": [{"type": "string"}, {"type": "array", "items": {"type": "integer", "minimum": 0, "maximum": 255}}]}
                        },
                        "required": ["path", "kind"]
                    }
                }
            },
            "required": ["project", "base_snapshot", "effects"]
        }),
        "project_candidate" | "project_resolve_dependencies" => serde_json::json!({
            "type": "object",
            "properties": {"candidate": {"type": "string"}},
            "required": ["candidate"]
        }),
        "project_merge_candidates" => serde_json::json!({
            "type": "object",
            "properties": {
                "left_candidate": {"type": "string"},
                "right_candidate": {"type": "string"}
            },
            "required": ["left_candidate", "right_candidate"]
        }),
        "project_validate_candidate" => serde_json::json!({
            "type": "object",
            "properties": {
                "candidate": {"type": "string"},
                "profile": {
                    "type": "string",
                    "enum": [
                        "php-domain-focused",
                        "project-import-focused",
                        "rust-workspace",
                        "project-ingestion-e2e"
                    ]
                }
            },
            "required": ["candidate"]
        }),
        _ => serde_json::json!({"type": "object"}),
    }
}
