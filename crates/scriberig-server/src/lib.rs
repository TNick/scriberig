//! Adapts remote protocol requests and events onto shared Scriberig application operations.

use std::error::Error;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

/// Serve the development health endpoint over HTTP.
///
/// # Parameters
/// - `host`: Interface address to bind.
/// - `port`: TCP port to bind.
/// - `database_path`: SQLite file to initialize before accepting requests.
///
/// # Returns
/// Unit after the listener stops accepting connections.
///
/// # Errors
/// Returns an error when the database, listener, or a client connection fails.
pub fn serve(
    host: &str,
    port: u16,
    database_path: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    scriberig_store_sqlite::initialize_development_database(database_path)?;
    let listener = TcpListener::bind((host, port))?;
    println!("Scriberig API listening on http://{host}:{port}");
    println!("SQLite database: {}", database_path.display());

    for connection in listener.incoming() {
        handle_connection(connection?)?;
    }
    Ok(())
}

/// Handle one HTTP request from the development web proxy.
///
/// # Parameters
/// - `stream`: Connected client socket.
///
/// # Returns
/// Unit after the HTTP response is written.
///
/// # Errors
/// Returns an error when the request or response cannot be read or written.
fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let mut request = [0_u8; 2048];
    let bytes_read = stream.read(&mut request)?;
    let request_line = std::str::from_utf8(&request[..bytes_read])
        .ok()
        .and_then(|text| text.lines().next())
        .unwrap_or_default();
    let (status, body) = if request_line.starts_with("GET /api/health ") {
        ("200 OK", r#"{"status":"ready","database":"ready"}"#)
    } else {
        ("404 Not Found", r#"{"error":"not found"}"#)
    };

    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}
