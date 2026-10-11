//! Implements the small JSON HTTP adapter used by the local web application.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::FromStr;

use scriberig_app::{ApplicationContext, InspectRequest, SnapshotResolution};
use scriberig_core::{EntityId, SnapshotId};
use serde_json::{Value, json};

use crate::development::DevelopmentFixture;
use crate::dispatch::Dispatcher;
use crate::server::ApplicationService;

/// Shared state for one local HTTP server.
pub struct HttpState<S> {
    /// Application boundary used for semantic inspection.
    pub service: S,
    /// Development identities advertised to the browser.
    pub fixture: DevelopmentFixture,
}

/// Handles one local HTTP connection.
///
/// # Parameters
/// - `stream`: Connected client socket.
/// - `state`: Application and fixture state.
///
/// # Returns
/// I/O result after one response is written.
///
/// # Errors
/// Returns an I/O error when the request or response cannot be read or written.
pub fn handle_connection<S: ApplicationService>(
    mut stream: TcpStream,
    state: &HttpState<S>,
) -> std::io::Result<()> {
    let mut request = vec![0_u8; 64 * 1024];
    let bytes_read = stream.read(&mut request)?;
    let text = String::from_utf8_lossy(&request[..bytes_read]);
    let Some((head, body)) = text.split_once("\r\n\r\n") else {
        return write_json(
            &mut stream,
            400,
            &json!({"code": "invalid_request", "message": "malformed HTTP request"}),
        );
    };
    let mut lines = head.lines();
    let Some(request_line) = lines.next() else {
        return write_json(
            &mut stream,
            400,
            &json!({"code": "invalid_request", "message": "missing request line"}),
        );
    };
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default();
    let path = request_parts.next().unwrap_or_default();
    let parsed = match (method, path) {
        ("GET", "/api/health") => Ok(json!({"status": "ready", "database": "ready"})),
        ("POST", "/api/protocol") => protocol(state, body),
        ("POST", "/api/protocol/capabilities") => Ok(capabilities(state)),
        ("POST", "/api/protocol/inspect") => inspect(state, body),
        ("POST", "/api/protocol/list") => list(state, body),
        _ => Err((
            404,
            json!({"code": "not_found", "message": "route not found"}),
        )),
    };
    match parsed {
        Ok(value) => write_json(&mut stream, 200, &value),
        Err((status, value)) => write_json(&mut stream, status, &value),
    }
}

/// Dispatches an authoritative protocol envelope through shared application semantics.
fn protocol<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let request: scriberig_protocol::RequestEnvelope =
        serde_json::from_str(body).map_err(|error| {
            (
                400,
                json!({"code": "invalid_request", "message": error.to_string()}),
            )
        })?;
    let response = Dispatcher::new(&state.service).dispatch(request);
    serde_json::to_value(response).map_err(|error| {
        (
            500,
            json!({"code": "internal", "message": error.to_string()}),
        )
    })
}

/// Returns the capabilities required for development bootstrap.
fn capabilities<S>(state: &HttpState<S>) -> Value {
    json!({
        "protocolMajor": 1,
        "features": ["inspect", "list"],
        "capabilities": [
            {"id": "inspect.entity", "description": "Inspect one entity", "available": true},
            {"id": "list.children", "description": "List root children", "available": true}
        ],
        "root": {
            "entityId": state.fixture.root_entity.to_string(),
            "snapshotId": state.fixture.snapshot.to_string()
        }
    })
}

/// Maps one browser inspection request through the application boundary.
fn inspect<S: ApplicationService>(state: &HttpState<S>, body: &str) -> Result<Value, (u16, Value)> {
    let value: Value = serde_json::from_str(body).map_err(|error| {
        (
            400,
            json!({"code": "invalid_request", "message": error.to_string()}),
        )
    })?;
    let entity_text = value
        .get("entityId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let entity = EntityId::from_str(entity_text).map_err(|_| {
        (
            400,
            json!({"code": "invalid_request", "message": "invalid entity id"}),
        )
    })?;
    let snapshot = snapshot_resolution(&value, state.fixture.snapshot)?;
    let result = state
        .service
        .inspect(
            &ApplicationContext::empty(),
            &InspectRequest {
                entity,
                snapshot,
                detail: scriberig_app::DetailLevel::Compact,
            },
        )
        .map_err(|error| {
            (
                404,
                json!({"code": error.category(), "message": error.to_string()}),
            )
        })?;
    Ok(object_description(
        &result.entity.to_string(),
        result.snapshot,
        result.revision,
        result.object_type.map(|value| value.to_string()),
        result.label.as_deref().or(result.description.as_deref()),
        None,
    ))
}

/// Returns one bounded page of development root children.
fn list<S: ApplicationService>(state: &HttpState<S>, body: &str) -> Result<Value, (u16, Value)> {
    let value: Value = serde_json::from_str(body).map_err(|error| {
        (
            400,
            json!({"code": "invalid_request", "message": error.to_string()}),
        )
    })?;
    let entity = value
        .get("entityId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if entity != state.fixture.root_entity.to_string() {
        return Ok(json!({"items": [], "nextCursor": null}));
    }
    let items = state
        .fixture
        .child_entities
        .iter()
        .map(|child| {
            object_description(
                child.to_string().as_str(),
                Some(state.fixture.snapshot),
                None,
                Some("scriberig.demo.entity".to_owned()),
                Some("Development entity"),
                None,
            )["summary"]
                .clone()
        })
        .collect::<Vec<_>>();
    Ok(json!({"items": items, "nextCursor": null}))
}

/// Resolves the browser snapshot selector without silently changing exact IDs.
///
/// # Parameters
/// - `value`: Browser request containing a snapshot selector.
/// - `current`: Current development snapshot used for the `current` selector.
///
/// # Returns
/// The application snapshot resolution.
///
/// # Errors
/// Returns a transport status and structured error body for invalid selectors.
fn snapshot_resolution(
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

/// Builds the public semantic object description shape consumed by the web client.
///
/// # Parameters
/// - `entity`: Entity identifier to expose.
/// - `snapshot`: Snapshot identity, when available.
/// - `revision`: Revision identity, when available.
/// - `object_type`: Semantic type, when available.
/// - `label`: Compact display label, when available.
/// - `child_count`: Known child count, when available.
///
/// # Returns
/// JSON-compatible semantic summary data.
fn object_description(
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
///
/// # Returns
/// Unit after the response is written.
///
/// # Errors
/// Returns an I/O error when the response cannot be written.
fn write_json(stream: &mut TcpStream, status: u16, body: &Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(&body).unwrap_or_else(|_| b"{\"code\":\"internal\"}".to_vec());
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(&body)
}
