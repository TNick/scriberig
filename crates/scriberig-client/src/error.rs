//! Defines structured client transport and protocol failures.

use scriberig_protocol::ProtocolError;

/// Errors returned by semantic client methods.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClientError {
    /// Server returned a structured protocol error.
    Protocol(ProtocolError),
    /// Server returned a response variant unrelated to the request.
    UnexpectedResponse,
}

impl std::fmt::Display for ClientError {
    /// Formats a client error.
    ///
    /// # Parameters
    /// - `self`: Error to format.
    /// - `formatter`: Formatting destination.
    ///
    /// # Returns
    /// Formatting result.
    ///
    /// # Errors
    /// Returns a formatting error when the destination rejects output.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Protocol(error) => write!(formatter, "{:?}: {}", error.code, error.message),
            Self::UnexpectedResponse => formatter.write_str("unexpected protocol response"),
        }
    }
}

impl std::error::Error for ClientError {}
