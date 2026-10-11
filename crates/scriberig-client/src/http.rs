//! Provides a small dependency-free HTTP transport for the public protocol.

use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use scriberig_protocol::{
    ProtocolError, ProtocolErrorCode, ProtocolTransport, RequestEnvelope, Response,
    ResponseEnvelope,
};

/// A short-lived HTTP transport for one protocol endpoint.
#[derive(Clone, Debug)]
pub struct HttpTransport {
    /// Hostname or address sent in the HTTP request.
    host: String,
    /// TCP port of the protocol server.
    port: u16,
    /// HTTP path receiving protocol envelopes.
    path: String,
    /// Connection timeout used for each request.
    timeout: Duration,
    /// Last transport failure awaiting conversion by the semantic client.
    error: Arc<Mutex<Option<String>>>,
}

impl HttpTransport {
    /// Parses an `http://host:port` endpoint.
    ///
    /// # Errors
    /// Returns an error for unsupported schemes, missing hosts, or invalid
    /// ports.
    /// - `endpoint`: Endpoint URL to parse.
    ///
    /// # Returns
    /// A configured HTTP transport.
    pub fn from_endpoint(endpoint: &str) -> io::Result<Self> {
        let remainder = endpoint.strip_prefix("http://").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "TUI remote endpoints must use the http:// scheme",
            )
        })?;
        let (authority, path) = remainder.split_once('/').unwrap_or((remainder, ""));
        let (host, port) = authority.rsplit_once(':').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "remote endpoint must include a port",
            )
        })?;
        let port = port.parse::<u16>().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "remote endpoint has an invalid port",
            )
        })?;
        if host.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "remote endpoint is missing a host",
            ));
        }
        Ok(Self {
            host: host.to_owned(),
            port,
            path: if path.is_empty() {
                "/api/protocol".to_owned()
            } else {
                format!("/{path}")
            },
            timeout: Duration::from_secs(5),
            error: Arc::new(Mutex::new(None)),
        })
    }

    /// Creates a transport for a host and port using the protocol route.
    #[must_use]
    /// - `host`: Hostname or address to contact.
    /// - `port`: TCP port to contact.
    ///
    /// # Returns
    /// A configured HTTP transport.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            path: "/api/protocol".to_owned(),
            timeout: Duration::from_secs(5),
            error: Arc::new(Mutex::new(None)),
        }
    }

    /// Sends one JSON protocol envelope over a fresh HTTP connection.
    /// - `request`: Protocol request to send.
    ///
    /// # Returns
    /// The decoded protocol response.
    ///
    /// # Errors
    /// Returns a transport or serialization error.
    fn request(&self, request: &RequestEnvelope) -> io::Result<ResponseEnvelope> {
        let body = serde_json::to_vec(request).map_err(io::Error::other)?;
        let address = (self.host.as_str(), self.port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::AddrNotAvailable, "host not found"))?;
        let mut stream = TcpStream::connect_timeout(&address, self.timeout)?;
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;
        write!(
            stream,
            "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.path,
            self.host,
            body.len()
        )?;
        stream.write_all(&body)?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response)?;
        let response = String::from_utf8(response).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "HTTP response was not UTF-8")
        })?;
        let (_, body) = response.split_once("\r\n\r\n").ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "HTTP response had no body")
        })?;
        serde_json::from_str(body).map_err(io::Error::other)
    }
}

impl ProtocolTransport for HttpTransport {
    /// Sends a protocol request and converts transport failures to safe errors.
    /// - `request`: Protocol request to send.
    ///
    /// # Returns
    /// The decoded protocol response.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        match self.request(&request) {
            Ok(response) => response,
            Err(error) => ResponseEnvelope {
                ..self.record_error(&error, request)
            },
        }
    }

    /// Returns the most recent network failure after a send.
    ///
    /// # Returns
    /// The most recent transport failure, if any.
    fn take_error(&self) -> Option<String> {
        self.error.lock().ok()?.take()
    }
}

impl HttpTransport {
    /// Records a transport error and builds a protocol-shaped fallback.
    ///
    /// # Parameters
    /// - `error`: Transport failure to record.
    /// - `request`: Request envelope that failed.
    ///
    /// # Returns
    /// A protocol-shaped error response.
    fn record_error(&self, error: &io::Error, request: RequestEnvelope) -> ResponseEnvelope {
        if let Ok(mut slot) = self.error.lock() {
            *slot = Some(error.to_string());
        }
        ResponseEnvelope {
            version: request.version,
            correlation_id: request.correlation_id,
            response: Response::Error(ProtocolError {
                code: ProtocolErrorCode::Internal,
                message: "remote transport unavailable".to_owned(),
                details: None,
                correlation_id: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HttpTransport;
    use crate::{Client, ClientError};

    /// Verifies endpoint parsing selects the public protocol route.
    #[test]
    fn parses_http_endpoint() {
        let transport = HttpTransport::from_endpoint("http://localhost:7337").expect("endpoint");
        assert_eq!(transport.host, "localhost");
        assert_eq!(transport.port, 7337);
    }

    /// Verifies unsupported endpoint schemes fail before any socket operation.
    #[test]
    fn rejects_non_http_endpoint() {
        let error = HttpTransport::from_endpoint("https://localhost:7337")
            .expect_err("https is not implemented by this transport");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    /// Verifies socket failures remain distinguishable from semantic errors.
    #[test]
    fn reports_transport_failure_separately() {
        let client = Client::new(HttpTransport::new("127.0.0.1", 0));
        let error = client.capabilities().expect_err("port 1 is unavailable");
        assert!(matches!(error, ClientError::Transport(_)));
    }
}
