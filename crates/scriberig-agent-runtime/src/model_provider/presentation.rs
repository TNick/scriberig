//! Produces safe structured tool arguments for detailed presentation.

/// Redacts schema-marked top-level properties before durable persistence.
///
/// - `arguments`: Original structured tool arguments.
/// - `schema`: Trusted tool parameter schema, when known.
///
/// Arguments with `writeOnly` or `x-sensitive` property values redacted.
/// # Returns
/// The operation result.
pub(super) fn redact_sensitive_arguments(
    arguments: &serde_json::Value,
    schema: Option<&serde_json::Value>,
) -> serde_json::Value {
    let mut safe = arguments.clone();
    redact_value(&mut safe, schema);
    safe
}

/// Applies sensitivity metadata recursively to one structured value.
///
/// - `value`: Argument value to sanitize in place.
/// - `schema`: Corresponding trusted JSON schema node.
fn redact_value(value: &mut serde_json::Value, schema: Option<&serde_json::Value>) {
    let Some(schema) = schema else {
        return;
    };
    if schema
        .get("writeOnly")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        || schema
            .get("x-sensitive")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    {
        *value = serde_json::Value::String("[REDACTED]".to_owned());
        return;
    }
    if let (Some(values), Some(properties)) = (
        value.as_object_mut(),
        schema
            .get("properties")
            .and_then(serde_json::Value::as_object),
    ) {
        for (name, nested) in values {
            redact_value(nested, properties.get(name));
        }
    }
    if let (Some(values), Some(item_schema)) = (value.as_array_mut(), schema.get("items")) {
        for item in values {
            redact_value(item, Some(item_schema));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::redact_sensitive_arguments;

    /// Keeps ordinary parameters and redacts schema-marked secret values.
    #[test]
    fn redacts_schema_marked_values_and_preserves_structure() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "token": {"type": "string", "writeOnly": true},
                "private_key": {"type": "string", "x-sensitive": true},
                "options": {
                    "type": "object",
                    "properties": {
                        "api_key": {"type": "string", "x-sensitive": true}
                    }
                }
            }
        });
        let arguments = serde_json::json!({
            "query": "find documents",
            "token": "FAKE_SECRET_MARKER_8391",
            "private_key": "ANOTHER_SECRET_MARKER_2741",
            "options": {"api_key": "NESTED_SECRET_MARKER_5739"}
        });

        let safe = redact_sensitive_arguments(&arguments, Some(&schema));

        assert_eq!(safe["query"], "find documents");
        assert_eq!(safe["token"], "[REDACTED]");
        assert_eq!(safe["private_key"], "[REDACTED]");
        assert_eq!(safe["options"]["api_key"], "[REDACTED]");
        assert!(!safe.to_string().contains("FAKE_SECRET_MARKER_8391"));
        assert!(!safe.to_string().contains("ANOTHER_SECRET_MARKER_2741"));
        assert!(!safe.to_string().contains("NESTED_SECRET_MARKER_5739"));
    }
}
