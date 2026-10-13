//! Adapts remote protocol operations to the TUI service contract.

#![allow(clippy::wildcard_imports)]

use super::service_remote_mapping::{
    map_remote_event, map_remote_operation, protocol_category, remote_error, session_from_response,
};
use super::*;

/// Adapts a protocol transport to TUI service operations.
pub struct RemoteAdapter<T> {
    /// Protocol client used for semantic requests.
    pub(super) client: scriberig_client::Client<T>,
    /// Last remote event cursor delivered to the TUI runtime.
    event_cursor: std::sync::Mutex<Option<u64>>,
}

impl<T> RemoteAdapter<T> {
    /// Creates a remote TUI adapter over a public protocol client.
    ///
    /// # Parameters
    /// - `client`: Protocol client used for remote TUI requests.
    ///
    /// # Returns
    /// A remote adapter backed by the client.
    #[must_use]
    pub fn new(client: scriberig_client::Client<T>) -> Self {
        Self {
            client,
            event_cursor: std::sync::Mutex::new(None),
        }
    }
}

impl<T: scriberig_protocol::ProtocolTransport> TuiService for RemoteAdapter<T> {
    /// Inspects an entity through the public protocol client.
    ///
    /// # Parameters
    /// - `_context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The decoded inspection result.
    ///
    /// # Errors
    /// Returns a protocol or response-decoding error.
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
    /// # Parameters
    /// - `context`: Authorization context for the remote request.
    /// - `entity`: Entity owning the semantic collection.
    /// - `snapshot`: Exact snapshot context.
    /// - `continuation`: Opaque continuation returned by the previous page.
    /// - `_context`: Authorization context for the remote request.
    ///
    /// # Returns
    /// Snapshot-bound Explorer rows and the next continuation, when present.
    ///
    /// # Errors
    /// Returns a structured remote protocol or application error.
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
    /// # Parameters
    /// - `context`: Authorization context for the remote request.
    /// - `snapshot`: Exact discovery scope.
    /// - `query`: User-authored search text.
    /// - `_context`: Authorization context for the remote request.
    ///
    /// # Returns
    /// A bounded TUI discovery model.
    ///
    /// # Errors
    /// Returns a structured remote protocol or application error.
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
    /// # Parameters
    /// - `_context`: Application context for capability access.
    ///
    /// # Returns
    /// Decoded runtime capabilities.
    ///
    /// # Errors
    /// Returns a protocol capability-discovery error.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        self.client
            .capabilities()
            .map_err(remote_error)
            .map(|response| {
                response
                    .capabilities
                    .into_iter()
                    .filter_map(|capability| {
                        Some(RuntimeCapability {
                            id: scriberig_app::CapabilityId::new(capability.id).ok()?,
                            description: capability.description,
                            available: capability.available,
                            scope: capability.domain,
                        })
                    })
                    .collect()
            })
    }

    /// Reads a remote operation lifecycle record through the public client.
    ///
    /// # Parameters
    /// - `_context`: Application context for the lookup.
    /// - `operation`: Operation identity to resolve.
    ///
    /// # Returns
    /// The remote operation record.
    ///
    /// # Errors
    /// Returns a remote protocol or decoding error.
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
    /// # Parameters
    /// - `_context`: Application context for cancellation.
    /// - `operation`: Operation identity to cancel.
    ///
    /// # Returns
    /// The cancelled remote operation record.
    ///
    /// # Errors
    /// Returns a remote protocol or decoding error.
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
    /// # Parameters
    /// - `_context`: Application context for event access.
    ///
    /// # Returns
    /// Decoded remote application events.
    ///
    /// # Errors
    /// Returns a remote protocol error.
    fn application_events(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::ApplicationEvent>, ApplicationError> {
        let cursor = self.event_cursor.lock().ok().and_then(|value| *value);
        let events = match self.client.subscribe_after(Vec::new(), cursor) {
            Ok(events) => events,
            Err(scriberig_client::ClientError::Protocol(error))
                if protocol_category(&error.code) == "cursor_too_old" =>
            {
                self.client
                    .subscribe_after(Vec::new(), None)
                    .map_err(remote_error)?
            }
            Err(error) => return Err(remote_error(error)),
        };
        if let Some(sequence) = events.iter().map(|event| event.sequence).max() {
            if let Ok(mut cursor) = self.event_cursor.lock() {
                *cursor = Some(sequence);
            }
        }
        Ok(events.into_iter().filter_map(map_remote_event).collect())
    }

    /// Reads and reconstructs the compact remote session view.
    ///
    /// # Parameters
    /// - `_context`: Authorized application context.
    /// - `session`: Stable session identity.
    ///
    /// # Returns
    /// A provider-neutral session value usable by the existing TUI view model.
    ///
    /// # Errors
    /// Returns a sanitized remote or identity-decoding error.
    fn read_session(
        &self,
        _context: &ApplicationContext,
        session: EntityId,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        let response = self
            .client
            .read_session(session.to_string())
            .map_err(remote_error)?;
        session_from_response(response)
    }

    /// Reads one remote branch subtree on demand.
    ///
    /// # Parameters
    /// - `_context`: Authorized application context.
    /// - `session`: Stable session identity.
    /// - `branch`: Branch whose activity is requested.
    ///
    /// # Returns
    /// The selected provider-neutral branch subtree.
    ///
    /// # Errors
    /// Returns a sanitized remote protocol failure.
    fn read_session_branch(
        &self,
        _context: &ApplicationContext,
        session: EntityId,
        branch: EntityId,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        let response = self
            .client
            .read_session_branch(session.to_string(), branch.to_string())
            .map_err(remote_error)?;
        session_from_response(response)
    }

    /// Submits a remote agent command through the shared operation protocol.
    ///
    /// # Parameters
    /// - `_context`: Authorized application context.
    /// - `session`: Stable session identity receiving the prompt.
    /// - `branch`: Optional target branch identity.
    /// - `prompt`: User-authored agent instruction.
    ///
    /// # Returns
    /// The accepted shared operation identity.
    ///
    /// # Errors
    /// Returns the sanitized remote protocol failure.
    fn submit_agent_command(
        &self,
        _context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        let operation = self
            .client
            .submit_agent_command(
                session.to_string(),
                branch.map(|value| value.to_string()),
                prompt,
            )
            .map_err(remote_error)?;
        scriberig_core::OperationId::from_str(&operation.operation.reference.id).map_err(|error| {
            ApplicationError::Internal {
                message: format!("remote response contained invalid operation identity: {error}"),
            }
        })
    }

    /// Submits remote agent work while retaining the server-provided branch handle.
    ///
    /// # Parameters
    /// - `_context`: Application context for the operation.
    /// - `session`: Stable session identity.
    /// - `branch`: Optional continuation branch.
    /// - `prompt`: User-authored instruction.
    ///
    /// # Returns
    /// Stable operation, session, and branch handles.
    ///
    /// # Errors
    /// Returns a sanitized remote protocol or identity error.
    fn submit_agent_handle(
        &self,
        _context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<super::AgentHandle, ApplicationError> {
        let accepted = match branch {
            Some(branch) => self
                .client
                .continue_agent_with_handles(session.to_string(), branch.to_string(), prompt)
                .map_err(remote_error)?,
            None => self
                .client
                .start_agent_with_handles(session.to_string(), prompt)
                .map_err(remote_error)?,
        };
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
}

#[cfg(test)]
#[path = "service_remote_tests.rs"]
mod tests;
