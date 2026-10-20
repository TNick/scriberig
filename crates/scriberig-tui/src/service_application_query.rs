//! Implements generic query service adapters for the TUI.

use super::*;

/// Converts the application model library into credential-safe wire descriptors.
///
/// - `library`: Validated application model library.
///
/// Protocol descriptors containing references but no resolved credentials.
/// # Returns
/// The operation result.
pub(in crate::service) fn model_library_response(
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
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// The inspection result.
    ///
    /// Returns the query service error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
