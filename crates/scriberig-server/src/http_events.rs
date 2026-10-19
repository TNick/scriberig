//! Implements the bounded HTTP Server-Sent Events observation endpoint.

use std::io::{self, Write};
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::to_string;

use crate::http::HttpState;
use crate::server::ApplicationService;

/// Handles a bounded replay-and-live SSE subscription.
///
/// The application event registry owns bounded retention and cursor validity.
/// The connection remains open for a bounded lease, polling the authoritative
/// event history and emitting heartbeats. Clients reconnect with the last id.
///
/// - `stream`: Connected HTTP client socket.
/// - `state`: Application service and fixture state.
/// - `path`: Request path, including optional query parameters.
///
/// I/O result after the replay and live lease are written.
///
/// Returns an I/O error when the response cannot be written.
pub(super) fn handle<S: ApplicationService>(
    stream: &mut TcpStream,
    state: &HttpState<S>,
    path: &str,
    origin: Option<&str>,
) -> io::Result<()> {
    let after = query_value(path, "after")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_default();
    if let Err(error) = state.service.events_after(&[], after) {
        return write_cursor_error(stream, &error, origin);
    }
    let cors = origin.map_or_else(String::new, |value| {
        format!("Access-Control-Allow-Origin: {value}\r\nVary: Origin\r\n")
    });
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n{cors}Cache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n"
    )?;
    stream.flush()?;
    let lease = Instant::now() + Duration::from_secs(30);
    let mut cursor = after;
    let mut heartbeat = Instant::now() + Duration::from_secs(10);
    while Instant::now() < lease {
        let events = state
            .service
            .events_after(&[], cursor)
            .map_err(|error| io::Error::other(format!("event subscription failed: {error}")))?;
        for event in events {
            cursor = cursor.max(event.sequence);
            let payload = to_string(&event).map_err(io::Error::other)?;
            write!(
                stream,
                "id: {}\nevent: protocol\ndata: {payload}\n\n",
                event.sequence
            )?;
        }
        if heartbeat <= Instant::now() {
            stream.write_all(b": heartbeat\n\n")?;
            heartbeat = Instant::now() + Duration::from_secs(10);
        }
        stream.flush()?;
        thread::sleep(Duration::from_millis(250));
    }
    Ok(())
}

/// Writes the explicit response for an expired event cursor.
///
/// - `stream`: Connected HTTP client socket.
/// - `error`: Cursor validation error.
/// - `origin`: Optional allowed CORS origin.
///
/// I/O result after the error response is written.
///
/// Returns the socket error when the response cannot be written.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn write_cursor_error(
    stream: &mut TcpStream,
    error: &scriberig_app::ApplicationError,
    origin: Option<&str>,
) -> io::Result<()> {
    let body = serde_json::json!({
        "code": error.category(),
        "message": error.to_string(),
    });
    let body = to_string(&body).unwrap_or_else(|_| "{\"code\":\"internal\"}".to_owned());
    let cors = origin.map_or_else(String::new, |value| {
        format!("Access-Control-Allow-Origin: {value}\r\nVary: Origin\r\n")
    });
    write!(
        stream,
        "HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\n{cors}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

/// Reads one URL query value without interpreting unrelated parameters.
///
/// - `path`: Request path and query string.
/// - `name`: Query parameter name.
///
/// The first matching raw value.
fn query_value<'a>(path: &'a str, name: &str) -> Option<&'a str> {
    path.split_once('?')?
        .1
        .split('&')
        .filter_map(|part| part.split_once('='))
        .find_map(|(key, value)| (key == name).then_some(value))
}
