//! Implements the small JSON HTTP adapter used by the local web application.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use scriberig_app::{ApplicationContext, InspectRequest, Principal, SnapshotResolution};
use scriberig_core::{EntityId, SnapshotId};
use serde_json::{Value, json};

use crate::development::DevelopmentFixture;
use crate::dispatch::Dispatcher;
use crate::server::ApplicationService;
use scriberig_store_sqlite::SqliteStore;

#[path = "http_helpers.rs"]
mod http_helpers;
use http_helpers::{object_description, write_json};
#[path = "http_events.rs"]
mod http_events;
#[path = "http_migration.rs"]
mod http_migration;
#[path = "http_project_transport.rs"]
mod http_project_transport;
#[path = "http_protocol.rs"]
mod http_protocol;
pub use http_project_transport::InProcessProjectTransport;
#[path = "http_request.rs"]
mod http_request;
#[path = "http_session.rs"]
mod http_session;
#[path = "project_routes.rs"]
mod project_routes;

use http_request::{allowed_origin, header_value, request_id};

/// Shared state for one local HTTP server.
pub struct HttpState<S> {
    /// Application boundary used for semantic inspection.
    pub service: S,
    /// Trusted identity for this single-user local transport instance.
    pub principal: Principal,
    /// Development identities advertised to the browser.
    pub fixture: DevelopmentFixture,
    /// Optional exact browser origin allowed by the server.
    pub allowed_origin: Option<String>,
    /// Optional project catalog used by onboarding routes.
    pub project_store: Option<Arc<SqliteStore>>,
    /// Canonical filesystem roots permitted for project scans and relinks.
    pub allowed_project_roots: Vec<PathBuf>,
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
    const MAX_REQUEST_BYTES: usize = 64 * 1024;
    let mut request = vec![0_u8; 64 * 1024];
    let bytes_read = stream.read(&mut request)?;
    let text = String::from_utf8_lossy(&request[..bytes_read]);
    let Some((head, body)) = text.split_once("\r\n\r\n") else {
        return write_json(
            &mut stream,
            400,
            &json!({"code": "invalid_request", "message": "malformed HTTP request"}),
            None,
        );
    };
    if let Some(length) = header_value(head, "content-length") {
        let length = length.parse::<usize>().unwrap_or(MAX_REQUEST_BYTES + 1);
        if length > MAX_REQUEST_BYTES {
            return write_json(
                &mut stream,
                413,
                &json!({"code": "request_too_large", "message": "request body exceeds 64 KiB"}),
                None,
            );
        }
        if length > body.len() {
            return write_json(
                &mut stream,
                400,
                &json!({"code": "invalid_request", "message": "incomplete request body"}),
                None,
            );
        }
    }
    let mut lines = head.lines();
    let Some(request_line) = lines.next() else {
        return write_json(
            &mut stream,
            400,
            &json!({"code": "invalid_request", "message": "missing request line"}),
            None,
        );
    };
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default();
    let path = request_parts.next().unwrap_or_default();
    let request_id = request_id(head);
    let origin = allowed_origin(head, state.allowed_origin.as_deref());
    let typed_protocol = head.lines().skip(1).any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            (name.eq_ignore_ascii_case("accept")
                && value.trim() == "application/vnd.scriberig.protocol+json")
                || (name.eq_ignore_ascii_case("x-scriberig-protocol") && value.trim() == "1")
        })
    });
    if method == "GET"
        && (path.starts_with("/api/protocol/events") || path.starts_with("/protocol/events"))
    {
        return http_events::handle(&mut stream, state, path, origin.as_deref());
    }
    if method == "POST" && typed_protocol {
        if let Some(operation) = path
            .strip_prefix("/protocol/")
            .or_else(|| path.strip_prefix("/api/protocol/"))
        {
            let result = http_protocol::dispatch(state, operation, body, request_id);
            return match result {
                Ok(value) => write_json(&mut stream, 200, &value, origin.as_deref()),
                Err((status, value)) => write_json(&mut stream, status, &value, origin.as_deref()),
            };
        }
    }
    let parsed = match (method, path) {
        ("GET", "/api/health") => Ok(json!({"status": "ready", "database": "ready"})),
        ("GET", "/api/projects") => project_routes::list(state),
        ("POST", "/api/projects/scan") => project_routes::scan(state, body),
        ("POST", "/api/projects/import") => project_routes::import(state, body),
        ("POST", "/api/projects/relink") => project_routes::relink(state, body),
        ("POST", "/api/protocol") => protocol(state, body),
        ("POST", "/api/protocol/project") => project_routes::direct(state, body),
        ("POST", "/api/protocol/session") => http_session::read(state, body),
        ("POST", "/api/protocol/capabilities") => Ok(capabilities(state)),
        ("POST", "/api/protocol/inspect") => inspect(state, body),
        ("POST", "/api/protocol/list") => list(state, body),
        ("POST", "/api/protocol/migration") => http_migration::migration(state, body),
        ("POST", "/api/protocol/subscribe") => subscribe(state, body),
        ("POST", "/api/protocol/command") => command(state, body),
        _ => Err((
            404,
            json!({"code": "not_found", "message": "route not found"}),
        )),
    };
    match parsed {
        Ok(value) => write_json(&mut stream, 200, &value, origin.as_deref()),
        Err((status, value)) => write_json(&mut stream, status, &value, origin.as_deref()),
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
    if let scriberig_protocol::Request::Project(project) = request.request.clone() {
        return serde_json::to_value(project_routes::protocol(
            state,
            &project,
            request.correlation_id,
        ))
        .map_err(|error| {
            (
                500,
                json!({"code": "internal", "message": error.to_string()}),
            )
        });
    }
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

/// Returns buffered public events for the browser model subscription.
fn subscribe<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let value: Value = serde_json::from_str(body).map_err(|error| {
        (
            400,
            json!({"code": "invalid_request", "message": error.to_string()}),
        )
    })?;
    let event_kinds = value
        .get("eventKinds")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let events = state.service.events(&event_kinds).map_err(|error| {
        (
            503,
            json!({"code": error.category(), "message": error.to_string()}),
        )
    })?;
    serde_json::to_value(events).map_err(|error| {
        (
            500,
            json!({"code": "internal", "message": error.to_string()}),
        )
    })
}

/// Executes one declared application command for browser clients.
fn command<S: ApplicationService>(state: &HttpState<S>, body: &str) -> Result<Value, (u16, Value)> {
    let value: Value = serde_json::from_str(body).map_err(|error| {
        (
            400,
            json!({"code": "invalid_request", "message": error.to_string()}),
        )
    })?;
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let payload = value.get("payload").unwrap_or(&Value::Null);
    let operation = state
        .service
        .command(&ApplicationContext::empty(), kind, payload)
        .map_err(|error| {
            (
                400,
                json!({"code": error.category(), "message": error.to_string()}),
            )
        })?;
    serde_json::to_value(operation).map_err(|error| {
        (
            500,
            json!({"code": "internal", "message": error.to_string()}),
        )
    })
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

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
