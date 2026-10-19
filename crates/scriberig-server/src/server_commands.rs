//! Decodes native server commands.

use super::{
    ApplicationError, ApplicationRequest, ApplyRequest, BranchRequest, CommitRequest, DiffRequest,
    OperationKind, RenderRequest,
};
use scriberig_app::DomainOperationRequest;

/// Decodes a protocol command into an application command.
/// - `super`: Parent-module imports used by this helper.
/// - `kind`: Command kind to decode.
/// - `payload`: Serialized command payload.
///
/// The decoded application command.
///
/// Returns an error when the payload is invalid.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn decode_command(
    kind: &str,
    payload: &serde_json::Value,
) -> Result<(OperationKind, ApplicationRequest), ApplicationError> {
    /// Implements `invalid`.
    /// - `kind`: Argument named `kind`.
    /// - `error`: Argument named `error`.
    ///
    /// # Returns
    /// The operation result.
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
        "domain_operation" => decode_domain_operation(payload)
            .map(|request| {
                (
                    OperationKind::DomainOperation,
                    ApplicationRequest::DomainOperation(request),
                )
            })
            .map_err(|error| invalid(kind, error)),
        "render" => serde_json::from_value::<RenderRequest>(payload.clone())
            .map(|request| (OperationKind::Render, ApplicationRequest::Render(request)))
            .map_err(|error| invalid(kind, error)),
        _ => Err(ApplicationError::UnsupportedOperation {
            operation: format!("command {kind}"),
        }),
    }
}

/// Converts a protocol domain operation payload to the application contract.
///
/// - `payload`: JSON-encoded typed domain operation command.
///
/// Exact application request with parsed semantic identifiers.
///
/// Returns a decoding error when an identifier or argument is invalid.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn decode_domain_operation(payload: &serde_json::Value) -> Result<DomainOperationRequest, String> {
    let command: scriberig_protocol::DomainOperationCommand =
        serde_json::from_value(payload.clone()).map_err(|error| error.to_string())?;
    let entity = command
        .entity
        .id
        .parse::<scriberig_core::EntityId>()
        .map_err(|error| error.to_string())?;
    let snapshot = command
        .snapshot
        .id
        .parse::<scriberig_core::SnapshotId>()
        .map_err(|error| error.to_string())?;
    let revision = command
        .revision
        .map(|value| value.parse::<scriberig_core::RevisionId>())
        .transpose()
        .map_err(|error| error.to_string())?;
    let branch = command
        .branch
        .map(|value| value.parse::<scriberig_core::RootName>())
        .transpose()
        .map_err(|error| error.to_string())?;
    let arguments = command
        .arguments
        .into_iter()
        .map(|(key, value)| {
            let value = if let Some(text) = value.as_str() {
                scriberig_query::QueryValue::Text(text.to_owned())
            } else if let Some(integer) = value.as_i64() {
                scriberig_query::QueryValue::Integer(integer)
            } else if let Some(boolean) = value.as_bool() {
                scriberig_query::QueryValue::Boolean(boolean)
            } else {
                return Err(format!(
                    "invalid argument {key}: expected a string, integer, or boolean"
                ));
            };
            Ok((key, value))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(DomainOperationRequest {
        capability: scriberig_app::CapabilityId::new(command.capability).map_err(str::to_owned)?,
        subject: scriberig_query::Selection {
            entity,
            snapshot,
            revision,
            relation_path: Vec::new(),
            projection_anchor: None,
        },
        context: scriberig_app::BranchContext {
            base_snapshot: snapshot,
            branch,
        },
        arguments,
    })
}

#[cfg(test)]
mod tests {
    use super::{ApplicationRequest, OperationKind, decode_command};

    /// Decodes exact typed domain operation payloads at the protocol boundary.
    #[test]
    fn domain_operation_command_preserves_exact_scope_and_arguments() {
        let entity = scriberig_core::EntityId::new();
        let snapshot = scriberig_core::SnapshotId::new([1; 32]);
        let payload = serde_json::json!({
            "capability": "code.add_function",
            "entity": {"id": entity.to_string()},
            "snapshot": {"id": snapshot.to_string()},
            "revision": null,
            "branch": "root_working",
            "arguments": {"name": "greet", "exported": true, "arity": 1}
        });
        let (kind, request) =
            decode_command("domain_operation", &payload).expect("decode semantic domain command");
        assert_eq!(kind, OperationKind::DomainOperation);
        let ApplicationRequest::DomainOperation(request) = request else {
            panic!("decoded incompatible application request");
        };
        assert_eq!(request.capability.as_str(), "code.add_function");
        assert_eq!(request.subject.entity, entity);
        assert_eq!(request.context.base_snapshot, snapshot);
        assert_eq!(request.context.branch.unwrap().as_str(), "working");
        assert_eq!(request.arguments.len(), 3);
    }

    /// Rejects non-scalar domain arguments before application dispatch.
    #[test]
    fn domain_operation_rejects_structured_arguments() {
        let entity = scriberig_core::EntityId::new();
        let snapshot = scriberig_core::SnapshotId::new([1; 32]);
        let payload = serde_json::json!({
            "capability": "code.add_function",
            "entity": {"id": entity.to_string()},
            "snapshot": {"id": snapshot.to_string()},
            "arguments": {"body": {"nested": true}}
        });
        assert!(decode_command("domain_operation", &payload).is_err());
    }
}
