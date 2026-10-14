//! Defines bounded JSON schemas for local control tools.

/// Defines precise JSON inputs for project bootstrap tools.
///
/// # Parameters
/// - `name`: Registered control operation.
/// - `super`: Parent module containing the child runtime catalog.
///
/// # Returns
/// The provider-visible JSON Schema for the operation's bounded arguments.
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
