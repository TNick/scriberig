//! Provides reusable HTTP response formatting helpers.

#![allow(clippy::wildcard_imports)]

use super::*;

/// Builds the public semantic object description shape consumed by the web client.
///
/// # Parameters
/// - `entity`: Entity identifier to expose.
/// - `snapshot`: Snapshot identity, when available.
/// - `revision`: Revision identity, when available.
/// - `object_type`: Semantic type, when available.
/// - `label`: Compact display label, when available.
/// - `child_count`: Known child count, when available.
/// - `super`: Parent-module visibility marker.
///
/// # Returns
/// JSON-compatible semantic summary data.
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
/// # Parameters
/// - `stream`: Connected client socket.
/// - `status`: HTTP status code.
/// - `body`: JSON response body.
/// - `origin`: Optional allowed CORS origin.
/// - `super`: Parent-module visibility marker.
///
/// # Returns
/// Unit after the response is written.
///
/// # Errors
/// Returns an I/O error when the response cannot be written.
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
