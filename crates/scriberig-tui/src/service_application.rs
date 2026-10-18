//! Adapts the application query service to the TUI service contract.

#![allow(clippy::wildcard_imports)]

use super::*;
use scriberig_app::{AgentConsoleBackend, SessionBackend};

/// Adapts an application query service to TUI operations.
pub struct ApplicationAdapter<S> {
    pub(super) service: S,
    capabilities: Vec<RuntimeCapability>,
}

impl<S> ApplicationAdapter<S> {
    /// Creates a TUI adapter around a public application service.
    ///
    /// # Parameters
    /// - `service`: Public query service.
    /// - `capabilities`: Runtime capabilities exposed by the service.
    ///
    /// # Returns
    /// A configured TUI adapter.
    #[must_use]
    pub fn new(service: S, capabilities: Vec<RuntimeCapability>) -> Self {
        Self {
            service,
            capabilities,
        }
    }

    /// Borrows the wrapped application service.
    #[must_use]
    pub const fn service(&self) -> &S {
        &self.service
    }
}

impl<S: QueryService + SessionBackend + AgentConsoleBackend> TuiService for ApplicationAdapter<S> {
    /// Delegates inspection through the public query service.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The inspection result.
    ///
    /// # Errors
    /// Returns the query service error.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        self.service.inspect(
            context,
            &InspectRequest {
                entity,
                snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
                detail: scriberig_app::DetailLevel::Detailed,
            },
        )
    }

    /// Returns the capabilities supplied by the composition root.
    ///
    /// # Parameters
    /// - `_context`: Application context for capability access.
    ///
    /// # Returns
    /// The configured runtime capabilities.
    ///
    /// # Errors
    /// This adapter does not return a capability error.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        Ok(self.capabilities.clone())
    }

    /// Reads a session through the public application boundary.
    ///
    /// # Parameters
    /// - `context`: Caller authorization context.
    /// - `session`: Stable session identity.
    ///
    /// # Returns
    /// The requested session snapshot.
    ///
    /// # Errors
    /// Returns the application session error.
    fn read_session(
        &self,
        context: &ApplicationContext,
        session: EntityId,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        self.service
            .read_session(context, &scriberig_app::SessionRequest { session })
    }

    /// Reads safe expanded detail through the application-owned session service.
    ///
    /// # Parameters
    /// - `context`: Caller authorization context.
    /// - `session`: Session containing the result.
    /// - `branch`: Branch containing the result.
    /// - `result`: Stable tool-result identity.
    ///
    /// # Returns
    /// Bounded allowlisted result detail.
    ///
    /// # Errors
    /// Returns the application authorization or lookup error.
    fn tool_result_detail(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: EntityId,
        result: EntityId,
    ) -> Result<scriberig_protocol::ToolResultDetailResponse, ApplicationError> {
        self.service
            .tool_result_detail(context, session, branch, result)
            .map(|detail| scriberig_protocol::ToolResultDetailResponse {
                result: detail.result.to_string(),
                detail: detail.detail,
                truncated: detail.truncated,
            })
    }

    /// Projects the application's safe model library into public protocol DTOs.
    ///
    /// # Parameters
    /// - `context`: Caller authorization context.
    ///
    /// # Returns
    /// Credential-safe model descriptors and profiles.
    ///
    /// # Errors
    /// Returns the application model-library error.
    fn model_library(
        &self,
        context: &ApplicationContext,
    ) -> Result<scriberig_protocol::ModelLibraryResponse, ApplicationError> {
        let library = self.service.agent_model_library(context)?;
        Ok(model_library_response(library))
    }

    /// Updates future model selection through the shared application service.
    ///
    /// # Parameters
    /// - `context`: Caller authorization context.
    /// - `request`: Session and revision-checked profile selection.
    ///
    /// # Returns
    /// The authoritative updated execution selection.
    ///
    /// # Errors
    /// Returns the shared application validation or persistence error.
    fn set_session_model(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::SessionModelSelectionRequest,
    ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
        self.service.set_session_model(context, request)
    }

    /// Updates future thinking selection through the shared application service.
    ///
    /// # Parameters
    /// - `context`: Caller authorization context.
    /// - `request`: Session and revision-checked thinking selection.
    ///
    /// # Returns
    /// The authoritative updated execution selection.
    ///
    /// # Errors
    /// Returns the shared application validation or persistence error.
    fn set_session_thinking_level(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::SessionThinkingSelectionRequest,
    ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
        self.service.set_session_thinking_level(context, request)
    }

    /// Submits an agent command through the public application boundary.
    ///
    /// # Parameters
    /// - `context`: Caller authorization context.
    /// - `session`: Target session identity.
    /// - `branch`: Optional target branch.
    /// - `prompt`: User-authored command text.
    ///
    /// # Returns
    /// The accepted operation identity.
    ///
    /// # Errors
    /// Returns the application session error.
    fn submit_agent_command(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        self.service.submit_agent_command(
            context,
            &scriberig_app::AgentCommandRequest {
                session,
                branch,
                prompt: prompt.to_owned(),
                selection: None,
            },
        )
    }

    /// Submits local agent work and resolves the accepted root branch handle.
    ///
    /// # Parameters
    /// - `context`: Application context for the operation.
    /// - `session`: Stable session identity.
    /// - `branch`: Optional continuation branch.
    /// - `prompt`: User-authored instruction.
    ///
    /// # Returns
    /// Stable operation, session, and branch handles.
    ///
    /// # Errors
    /// Returns the application submission or session-read error.
    fn submit_agent_handle(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<super::AgentHandle, ApplicationError> {
        let operation = self.submit_agent_command(context, session, branch, prompt)?;
        let branch = branch.or_else(|| {
            self.read_session(context, session)
                .ok()
                .and_then(|value| value.branches.last().map(|value| value.id))
        });
        Ok(super::AgentHandle {
            operation,
            session,
            branch,
            workspace: None,
        })
    }
}

/// Converts the application model library into credential-safe wire descriptors.
///
/// # Parameters
/// - `library`: Validated application model library.
///
/// # Returns
/// Protocol descriptors containing references but no resolved credentials.
fn model_library_response(
    library: scriberig_app::ModelLibrary,
) -> scriberig_protocol::ModelLibraryResponse {
    scriberig_protocol::ModelLibraryResponse {
        models: library
            .models
            .into_iter()
            .map(|model| scriberig_protocol::PublicModelDescriptor {
                id: model.id.as_str().to_owned(),
                display_name: model.display_name,
                provider_profile: model.provider_profile,
                provider_model_id: model.provider_model_id,
                capabilities: scriberig_protocol::PublicModelCapabilities {
                    streaming: model.capabilities.streaming,
                    tool_calls: model.capabilities.tool_calls,
                    structured_output: model.capabilities.structured_output,
                },
                thinking_levels: model
                    .thinking_levels
                    .into_iter()
                    .map(|level| scriberig_protocol::PublicThinkingLevelDescriptor {
                        id: level.id.as_str().to_owned(),
                        label: level.label,
                        description: level.description,
                        order: level.order,
                    })
                    .collect(),
                default_thinking_level: model
                    .default_thinking_level
                    .map(|level| level.as_str().to_owned()),
            })
            .collect(),
        profiles: library
            .profiles
            .into_iter()
            .map(|profile| scriberig_protocol::PublicModelProfile {
                id: profile.id,
                display_name: profile.display_name,
                model: profile.model.as_str().to_owned(),
                connection: profile.connection,
                credential: profile
                    .credential
                    .map(|credential| scriberig_protocol::ModelCredentialRef { id: credential.id }),
                default_thinking_level: profile
                    .default_thinking_level
                    .map(|level| level.as_str().to_owned()),
                enabled: profile.enabled,
            })
            .collect(),
    }
}

impl<S: QueryService> TuiService for S {
    /// Delegates inspection through a public query service.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The inspection result.
    ///
    /// # Errors
    /// Returns the query service error.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        self.inspect(
            context,
            &InspectRequest {
                entity,
                snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
                detail: scriberig_app::DetailLevel::Detailed,
            },
        )
    }
}

impl<S: QueryService> TuiModelRequestService for S {}
