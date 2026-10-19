//! Adapts remote protocol operations to the TUI service contract.

#![allow(clippy::wildcard_imports)]

#[macro_use]
#[path = "service_remote_plan_contract_methods.rs"]
mod plan_contract_methods;
#[macro_use]
#[path = "service_remote_session_methods.rs"]
mod session_methods;
use super::service_remote_mapping::{
    map_remote_operation, protocol_agent_selection, remote_error, session_from_response,
};
use super::service_remote_models;
use super::*;
use std::collections::BTreeSet;

#[path = "service_remote_plan_operations.rs"]
mod plan_operations;
#[path = "service_remote_events.rs"]
mod remote_events;

/// Adapts a protocol transport to TUI service operations.
pub struct RemoteAdapter<T> {
    /// Protocol client used for semantic requests.
    pub(super) client: scriberig_client::Client<T>,
    /// Last remote event cursor delivered to the TUI runtime.
    event_cursor: std::sync::Mutex<Option<u64>>,
    /// Session identities whose durable state can be refreshed after a cursor gap.
    known_sessions: std::sync::Mutex<BTreeSet<EntityId>>,
}

impl<T> RemoteAdapter<T> {
    /// Creates a remote TUI adapter over a public protocol client.
    ///
    /// - `client`: Protocol client used for remote TUI requests.
    ///
    /// A remote adapter backed by the client.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(client: scriberig_client::Client<T>) -> Self {
        Self {
            client,
            event_cursor: std::sync::Mutex::new(None),
            known_sessions: std::sync::Mutex::new(BTreeSet::new()),
        }
    }
}

impl<T: scriberig_protocol::ProtocolTransport> TuiService for RemoteAdapter<T> {
    /// Continues a remote conversation from an exact durable anchor.
    ///
    /// - `self`: Remote service adapter.
    /// - `_context`: Local context; remote authorization is transport-owned.
    /// - `session`: Owning session identity.
    /// - `anchor`: Exact replyable location.
    /// - `prompt`: User-authored follow-up.
    ///
    /// Authoritative operation, session, and continuation branch handles.
    ///
    /// Returns sanitized protocol or identity errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn continue_agent_at_handle(
        &self,
        _context: &ApplicationContext,
        session: EntityId,
        anchor: scriberig_session::ContinuationAnchor,
        prompt: &str,
    ) -> Result<super::AgentHandle, ApplicationError> {
        let anchor = super::service_remote_mapping::protocol_anchor(anchor);
        let accepted = self
            .client
            .continue_agent_at(session.to_string(), anchor, prompt, None)
            .map_err(remote_error)?;
        let operation =
            scriberig_core::OperationId::from_str(&accepted.operation.operation.reference.id)
                .map_err(|error| ApplicationError::Internal {
                    message: format!(
                        "remote response contained invalid operation identity: {error}"
                    ),
                })?;
        let branch = accepted
            .branch
            .as_ref()
            .map(|value| EntityId::from_str(&value.id))
            .transpose()
            .map_err(|error| ApplicationError::Internal {
                message: format!("remote response contained invalid branch identity: {error}"),
            })?;
        Ok(super::AgentHandle {
            operation,
            session,
            branch,
            workspace: None,
        })
    }

    /// Starts a new remote session through the shared agent protocol.
    ///
    /// - `self`: Remote service adapter.
    /// - `_context`: Local request context; remote authorization is transport-owned.
    /// - `prompt`: User-authored first instruction.
    /// - `selection`: Exact semantic scope for the initial model context.
    ///
    /// Authoritative operation, session, and root branch handles.
    ///
    /// Returns a sanitized protocol or identity error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn start_new_session_handle(
        &self,
        _context: &ApplicationContext,
        prompt: &str,
        selection: Option<scriberig_app::AgentSelection>,
    ) -> Result<super::AgentHandle, ApplicationError> {
        let accepted = self
            .client
            .start_new_session_agent(prompt, selection.as_ref().map(protocol_agent_selection))
            .map_err(remote_error)?;
        let operation =
            scriberig_core::OperationId::from_str(&accepted.operation.operation.reference.id)
                .map_err(|error| ApplicationError::Internal {
                    message: format!(
                        "remote response contained invalid operation identity: {error}"
                    ),
                })?;
        let session = EntityId::from_str(&accepted.session.id).map_err(|error| {
            ApplicationError::Internal {
                message: format!("remote response contained invalid session identity: {error}"),
            }
        })?;
        let branch = accepted
            .branch
            .as_ref()
            .map(|value| EntityId::from_str(&value.id))
            .transpose()
            .map_err(|error| ApplicationError::Internal {
                message: format!("remote response contained invalid branch identity: {error}"),
            })?;
        Ok(super::AgentHandle {
            operation,
            session,
            branch,
            workspace: None,
        })
    }

    /// Inspects an entity through the public protocol client.
    ///
    /// - `_context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// The decoded inspection result.
    ///
    /// Returns a protocol or response-decoding error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect_entity(
        &self,
        _context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        let selector = snapshot.map_or(scriberig_protocol::SnapshotSelector::Current, |value| {
            scriberig_protocol::SnapshotSelector::Exact(scriberig_protocol::SnapshotReference {
                id: value.to_string(),
            })
        });
        let response = self
            .client
            .inspect(entity.to_string(), selector, true)
            .map_err(remote_error)?;
        let resolved_entity = EntityId::from_str(&response.entity.id).map_err(|error| {
            ApplicationError::Internal {
                message: format!("remote response contained invalid entity identity: {error}"),
            }
        })?;
        let resolved_snapshot = SnapshotId::from_str(&response.snapshot.id).map_err(|error| {
            ApplicationError::Internal {
                message: format!("remote response contained invalid snapshot identity: {error}"),
            }
        })?;
        let revision = RevisionId::from_str(&response.revision.id).ok();
        Ok(InspectResult {
            entity: resolved_entity,
            revision,
            snapshot: Some(resolved_snapshot),
            object_type: response
                .object_type
                .and_then(|value| scriberig_core::ObjectType::new(value).ok()),
            label: response.summary,
            description: None,
            capabilities: Vec::new(),
            trust: None,
        })
    }

    /// Lists snapshot entities through the public remote query boundary.
    ///
    /// - `context`: Authorization context for the remote request.
    /// - `entity`: Entity owning the semantic collection.
    /// - `snapshot`: Exact snapshot context.
    /// - `continuation`: Opaque continuation returned by the previous page.
    /// - `_context`: Authorization context for the remote request.
    ///
    /// Snapshot-bound Explorer rows and the next continuation, when present.
    ///
    /// Returns a structured remote protocol or application error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn list_explorer(
        &self,
        _context: &ApplicationContext,
        entity: EntityId,
        snapshot: SnapshotId,
        continuation: Option<scriberig_query::ContinuationToken>,
    ) -> Result<crate::explorer::ExplorerPage, ApplicationError> {
        super::service_remote_mapping::list_explorer(&self.client, entity, snapshot, continuation)
    }

    /// Executes exact-text discovery through the public remote boundary.
    ///
    /// - `context`: Authorization context for the remote request.
    /// - `snapshot`: Exact discovery scope.
    /// - `query`: User-authored search text.
    /// - `_context`: Authorization context for the remote request.
    ///
    /// A bounded TUI discovery model.
    ///
    /// Returns a structured remote protocol or application error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn find_entities(
        &self,
        _context: &ApplicationContext,
        snapshot: SnapshotId,
        query: &str,
    ) -> Result<crate::view::find::FindModel, ApplicationError> {
        super::service_remote_mapping::find_entities(&self.client, snapshot, query)
    }

    /// Converts remote capability descriptors into application metadata.
    ///
    /// - `_context`: Application context for capability access.
    ///
    /// Decoded runtime capabilities.
    ///
    /// Returns a protocol capability-discovery error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        service_remote_models::runtime_capabilities(self)
    }

    /// Reads a remote operation lifecycle record through the public client.
    ///
    /// - `_context`: Application context for the lookup.
    /// - `operation`: Operation identity to resolve.
    ///
    /// The remote operation record.
    ///
    /// Returns a remote protocol or decoding error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn operation_status(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        let response = self
            .client
            .operation_status(operation.to_string())
            .map_err(remote_error)?;
        map_remote_operation(response.operation)
    }

    /// Requests remote cancellation and returns the reported lifecycle state.
    ///
    /// - `_context`: Application context for cancellation.
    /// - `operation`: Operation identity to cancel.
    ///
    /// The cancelled remote operation record.
    ///
    /// Returns a remote protocol or decoding error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn cancel_operation(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        let response = self
            .client
            .cancel_operation(operation.to_string())
            .map_err(remote_error)?;
        map_remote_operation(response.operation)
    }

    /// Polls public remote events for the runtime event/action path.
    ///
    /// - `_context`: Application context for event access.
    ///
    /// Decoded remote application events.
    ///
    /// Returns a remote protocol error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn application_events(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::ApplicationEvent>, ApplicationError> {
        remote_events::application_events(self)
    }

    /// Reads and reconstructs the compact remote session view.
    ///
    /// - `_context`: Authorized application context.
    /// - `session`: Stable session identity.
    ///
    /// A provider-neutral session value usable by the existing TUI view model.
    ///
    /// Returns a sanitized remote or identity-decoding error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_session(
        &self,
        _context: &ApplicationContext,
        session: EntityId,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        let response = self
            .client
            .read_session(session.to_string())
            .map_err(remote_error)?;
        if let Ok(mut known_sessions) = self.known_sessions.lock() {
            known_sessions.insert(session);
        }
        session_from_response(response)
    }

    remote_plan_contract_methods!();
    remote_session_methods!();
}

#[cfg(test)]
#[path = "service_remote_tests.rs"]
mod tests;
