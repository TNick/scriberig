//! Provides reusable HTTP response formatting helpers.

#![allow(clippy::wildcard_imports)]

use super::*;

/// Builds the public semantic object description shape consumed by the web client.
///
/// - `entity`: Entity identifier to expose.
/// - `snapshot`: Snapshot identity, when available.
/// - `revision`: Revision identity, when available.
/// - `object_type`: Semantic type, when available.
/// - `label`: Compact display label, when available.
/// - `child_count`: Known child count, when available.
/// - `super`: Parent-module visibility marker.
///
/// JSON-compatible semantic summary data.
/// # Returns
/// The operation result.
pub(super) fn object_description(
    entity: &str,
    snapshot: Option<SnapshotId>,
    revision: Option<scriberig_core::RevisionId>,
    object_type: Option<String>,
    label: Option<&str>,
    child_count: Option<usize>,
) -> Value {
    json!({
        "summary": {
            "ref": {"entityId": entity, "snapshotId": snapshot.map(|value| value.to_string()), "revisionId": revision.map(|value| value.to_string())},
            "typeId": object_type.unwrap_or_else(|| "scriberig.unknown".to_owned()),
            "label": label,
            "childCount": child_count
        }
    })
}

/// Writes a JSON response with a minimal HTTP/1.1 envelope.
///
/// - `stream`: Connected client socket.
/// - `status`: HTTP status code.
/// - `body`: JSON response body.
/// - `origin`: Optional allowed CORS origin.
/// - `super`: Parent-module visibility marker.
///
/// Unit after the response is written.
///
/// Returns an I/O error when the response cannot be written.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn write_json(
    stream: &mut TcpStream,
    status: u16,
    body: &Value,
    origin: Option<&str>,
) -> std::io::Result<()> {
    let body = serde_json::to_vec(&body).unwrap_or_else(|_| b"{\"code\":\"internal\"}".to_vec());
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    let cors = origin.map_or_else(String::new, |value| {
        format!("Access-Control-Allow-Origin: {value}\r\nVary: Origin\r\n")
    });
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\n{cors}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(&body)
}

/// Resolves the browser snapshot selector without silently changing exact IDs.
///
/// - `value`: Browser request containing a snapshot selector.
/// - `current`: Current development snapshot used for the `current` selector.
///
/// The application snapshot resolution.
///
/// Returns a transport status and structured error body for invalid selectors.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the selector is invalid.
pub(super) fn snapshot_resolution(
    value: &Value,
    current: SnapshotId,
) -> Result<SnapshotResolution, (u16, Value)> {
    match value
        .get("snapshot")
        .and_then(|item| item.get("kind"))
        .and_then(Value::as_str)
    {
        Some("exact") => {
            let id = value["snapshot"]["snapshotId"].as_str().unwrap_or_default();
            SnapshotId::from_str(id)
                .map(SnapshotResolution::At)
                .map_err(|_| {
                    (
                        400,
                        json!({"code": "invalid_request", "message": "invalid snapshot id"}),
                    )
                })
        }
        Some("current") | None => Ok(SnapshotResolution::At(current)),
        Some(_) => Err((
            400,
            json!({"code": "invalid_request", "message": "unsupported snapshot selector"}),
        )),
    }
}
