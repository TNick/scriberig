//! Defines structured client transport and protocol failures.

use scriberig_protocol::ProtocolError;

/// Errors returned by semantic client methods.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClientError {
    /// The protocol request could not reach its transport endpoint.
    Transport(String),
    /// Server returned a structured protocol error.
    Protocol(Box<ProtocolError>),
    /// A typed request could not be encoded for transport.
    Encoding(String),
    /// Server returned a response variant unrelated to the request.
    UnexpectedResponse,
}

impl std::fmt::Display for ClientError {
    /// Formats a client error.
    ///
    /// - `self`: Error to format.
    /// - `formatter`: Formatting destination.
    ///
    /// Formatting result.
    ///
    /// Returns a formatting error when the destination rejects output.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(message) => write!(formatter, "transport unavailable: {message}"),
            Self::Protocol(error) => write!(formatter, "{:?}: {}", error.code, error.message),
            Self::Encoding(message) => write!(formatter, "request encoding failed: {message}"),
            Self::UnexpectedResponse => formatter.write_str("unexpected protocol response"),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<ProtocolError> for ClientError {
    /// Boxes a protocol failure for compact client result values.
    ///
    /// - `error`: Structured protocol failure received from the server.
    ///
    /// A compact client error.
    /// # Returns
    /// The boxed protocol failure.
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(Box::new(error))
    }
}
