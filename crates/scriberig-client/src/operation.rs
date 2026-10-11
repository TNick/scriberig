//! Defines client-side operation polling helpers.

use scriberig_protocol::OperationStatusResponse;

/// Owns one operation reference for repeated status polling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationPoller {
    /// Stable operation identifier.
    pub operation_id: String,
}

impl OperationPoller {
    /// Creates a poller for one operation.
    ///
    /// # Parameters
    /// - `operation_id`: Stable operation identifier.
    ///
    /// # Returns
    /// A polling handle.
    #[must_use]
    pub fn new(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
        }
    }

    /// Accepts a status response from the semantic client.
    ///
    /// # Parameters
    /// - `response`: Status response to expose to callers.
    ///
    /// # Returns
    /// The contained protocol operation state.
    #[must_use]
    pub fn observe(&self, response: OperationStatusResponse) -> scriberig_protocol::Operation {
        response.operation
    }
}
