//! Decodes native server commands.

use super::{
    ApplicationError, ApplicationRequest, ApplyRequest, BranchRequest, CommitRequest, DiffRequest,
    OperationKind,
};

/// Decodes a protocol command into an application command.
/// - `super`: Parent-module imports used by this helper.
/// - `kind`: Command kind to decode.
/// - `payload`: Serialized command payload.
///
/// # Returns
/// The decoded application command.
///
/// # Errors
/// Returns an error when the payload is invalid.
pub(super) fn decode_command(
    kind: &str,
    payload: &serde_json::Value,
) -> Result<(OperationKind, ApplicationRequest), ApplicationError> {
    /// Implements `invalid`.
    /// - `kind`: Argument named `kind`.
    /// - `error`: Argument named `error`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    fn invalid(kind: &str, error: impl std::fmt::Display) -> ApplicationError {
        ApplicationError::InvalidRequest {
            operation: kind.to_owned(),
            message: error.to_string(),
        }
    }

    match kind {
        "apply" => serde_json::from_value::<ApplyRequest>(payload.clone())
            .map(|request| (OperationKind::Apply, ApplicationRequest::Apply(request)))
            .map_err(|error| invalid(kind, error)),
        "branch" => serde_json::from_value::<BranchRequest>(payload.clone())
            .map(|request| (OperationKind::Branch, ApplicationRequest::Branch(request)))
            .map_err(|error| invalid(kind, error)),
        "commit" => serde_json::from_value::<CommitRequest>(payload.clone())
            .map(|request| (OperationKind::Commit, ApplicationRequest::Commit(request)))
            .map_err(|error| invalid(kind, error)),
        "diff" => serde_json::from_value::<DiffRequest>(payload.clone())
            .map(|request| (OperationKind::Diff, ApplicationRequest::Diff(request)))
            .map_err(|error| invalid(kind, error)),
        _ => Err(ApplicationError::UnsupportedOperation {
            operation: format!("command {kind}"),
        }),
    }
}
