//! Shared protocol conversion helpers for the server dispatcher.

use scriberig_app::ApplicationError;
use scriberig_app::{
    ApplicationContext, Capability, CapabilityAction, CapabilityConstraints, CapabilityContext,
    CapabilityId, CapabilityScope, OperationKind,
};
use scriberig_protocol::{
    ArtifactEditCapability, EntityReference, InspectEntityResponse, ProtocolError,
    ProtocolErrorCode, RevisionReference, SessionReadResponse, SnapshotReference,
};

use crate::server::server_helpers::error_details;

/// Maps project artifact fidelity into the stable public wire capability.
///
/// - `capability`: Project-level capability attached to the exact source artifact.
///
/// The equivalent protocol capability for clients and product surfaces.
///
/// # Returns
/// The corresponding stable protocol capability.
pub fn map_artifact_edit_capability(
    capability: scriberig_project::ArtifactCapability,
) -> ArtifactEditCapability {
    match capability {
        scriberig_project::ArtifactCapability::Unsupported => ArtifactEditCapability::Unsupported,
        scriberig_project::ArtifactCapability::InspectOnly => ArtifactEditCapability::InspectOnly,
        scriberig_project::ArtifactCapability::ReadOnlyDerived => {
            ArtifactEditCapability::ReadOnlyDerived
        }
        scriberig_project::ArtifactCapability::LosslessEditable => {
            ArtifactEditCapability::LosslessEditable
        }
        scriberig_project::ArtifactCapability::LossAwareEditable => {
            ArtifactEditCapability::LossAwareEditable
        }
        scriberig_project::ArtifactCapability::EditableWithOpaquePreservation => {
            ArtifactEditCapability::EditableWithOpaquePreservation
        }
        scriberig_project::ArtifactCapability::RestrictedExternalProviderEdit => {
            ArtifactEditCapability::RestrictedExternalProviderEdit
        }
    }
}

/// Restricts a session response to one branch and its descendants.
///
/// - `response`: Durable session payload before filtering.
/// - `branch`: Branch identifier to retain in the projection.
///
/// The original session response reduced to the selected branch subtree.
/// # Returns
/// The operation result.
pub fn select_branch_subtree(response: SessionReadResponse, branch: &str) -> SessionReadResponse {
    let mut selected = std::collections::BTreeSet::from([branch.to_owned()]);
    let mut changed = true;
    while changed {
        changed = false;
        for candidate in &response.branches {
            if candidate
                .parent
                .as_ref()
                .is_some_and(|parent| selected.contains(parent))
                && selected.insert(candidate.id.clone())
            {
                changed = true;
            }
        }
    }

    let branches = response
        .branches
        .into_iter()
        .filter(|candidate| selected.contains(&candidate.id))
        .collect();

    SessionReadResponse {
        branches,
        ..response
    }
}

/// Converts an application inspection result to protocol references.
///
/// - `result`: Application inspection result.
///
/// Protocol inspection response with stable references.
/// # Returns
/// The operation result.
pub fn map_inspect(result: scriberig_app::InspectResult) -> InspectEntityResponse {
    InspectEntityResponse {
        entity: EntityReference {
            id: result.entity.to_string(),
        },
        revision: RevisionReference {
            entity: EntityReference {
                id: result.entity.to_string(),
            },
            id: result
                .revision
                .map_or_else(String::new, |value| value.to_string()),
        },
        snapshot: SnapshotReference {
            id: result
                .snapshot
                .map_or_else(String::new, |value| value.to_string()),
        },
        object_type: result.object_type.map(|value| value.to_string()),
        summary: result.description.or(result.label),
    }
}

/// Converts an application error without exposing restricted payloads.
///
/// - `error`: Application error to classify.
/// - `correlation_id`: Request correlation identity.
///
/// Structured protocol error.
/// # Returns
/// The operation result.
pub fn map_error(error: &ApplicationError, correlation_id: &str) -> ProtocolError {
    let code = match error.category() {
        "not_found" => ProtocolErrorCode::NotFound,
        "invalid_request" => ProtocolErrorCode::InvalidRequest,
        "permission_denied" => ProtocolErrorCode::PermissionDenied,
        "schema_violation" => ProtocolErrorCode::SchemaViolation,
        "stale_base_snapshot" => ProtocolErrorCode::StaleBaseSnapshot,
        "semantic_conflict" => ProtocolErrorCode::SemanticConflict,
        "validation_failure" => ProtocolErrorCode::ValidationFailure,
        "unsupported_operation" => ProtocolErrorCode::UnsupportedOperation,
        "unsupported_capability" => ProtocolErrorCode::UnsupportedCapability,
        "conflict" => ProtocolErrorCode::Conflict,
        "ambiguous" => ProtocolErrorCode::Ambiguous,
        "snapshot_unavailable" => ProtocolErrorCode::SnapshotUnavailable,
        "operation_failed" => ProtocolErrorCode::OperationFailed,
        "projection_out_of_sync" => ProtocolErrorCode::ProjectionOutOfSync,
        "operation_cancelled" => ProtocolErrorCode::OperationCancelled,
        "cursor_too_old" => ProtocolErrorCode::CursorTooOld,
        _ => ProtocolErrorCode::Internal,
    };
    ProtocolError {
        code,
        message: error.to_string(),
        details: Some(error_details(error)),
        correlation_id: Some(correlation_id.to_owned()),
    }
}

/// Builds the trusted server context for an exposed command family.
///
/// - `kind`: Stable command or capability identifier.
///
/// A context granting only the requested public command capability.
/// # Returns
/// The operation result.
pub(super) fn command_context(kind: &str) -> ApplicationContext {
    let mut capabilities = vec![CapabilityId::new(kind).expect("protocol command capability")];
    if matches!(kind, "agent_start" | "agent_continue" | "agent_plan") {
        capabilities.push(CapabilityId::new("session_branch_tree").expect("session capability"));
    }
    ApplicationContext {
        capabilities: CapabilityContext::new(capabilities),
        ..ApplicationContext::empty()
    }
}

/// Grants the trusted session-selection action for one exact session.
///
/// - `session`: Stable session identity from the protocol request.
///
/// Application context authorizing session model configuration.
/// # Returns
/// The operation result.
pub(super) fn session_selection_context(session: &str) -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext {
            granted: vec![
                CapabilityId::new("session_branch_tree").expect("session read capability"),
            ],
            grants: vec![Capability {
                scope: CapabilityScope::Session(session.to_owned()),
                actions: [CapabilityAction::ModifyBranch].into_iter().collect(),
                constraints: CapabilityConstraints {
                    operation: Some(OperationKind::Agent),
                    ..CapabilityConstraints::unrestricted()
                },
            }],
        },
        ..ApplicationContext::empty()
    }
}
