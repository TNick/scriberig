//! Exposes shared agent-profile resolution through the application facade.

use scriberig_core::SnapshotId;

use crate::{
    AgentProfileDescriptor, AgentProfileError, AgentProfileExecutionPreference,
    AgentProfileStoreResult, ApplicationContext, ApplicationError, ApplicationService,
    LegacyProfileMappingRequest, ProfileCandidate,
};

impl<B> ApplicationService<B> {
    /// Reads the exact current snapshot for shared semantic agent profiles.
    ///
    /// # Parameters
    /// - `self`: Shared application facade.
    ///
    /// # Returns
    /// The current profile root snapshot or its deterministic empty snapshot.
    ///
    /// # Errors
    /// Returns an unsupported-operation or profile storage error.
    pub fn current_agent_profile_snapshot(&self) -> Result<SnapshotId, AgentProfileError> {
        self.agent_profile_provider
            .as_ref()
            .ok_or_else(|| AgentProfileError::Unsupported("profile snapshot read".to_owned()))?
            .current_snapshot()
    }

    /// Resolves one current semantic profile, preserving the bootstrap fallback when absent.
    ///
    /// # Parameters
    /// - `self`: Shared application facade.
    /// - `context`: Trusted caller capabilities.
    /// - `identity`: Stable profile identity to resolve.
    ///
    /// # Returns
    /// The exact persisted profile, or `None` if no profile provider/value is configured.
    ///
    /// # Errors
    /// Returns authorization, storage, or profile decoding errors from the provider.
    pub fn resolve_agent_profile(
        &self,
        context: &ApplicationContext,
        identity: &str,
    ) -> Result<Option<AgentProfileDescriptor>, AgentProfileError> {
        self.agent_profile_provider
            .as_ref()
            .map_or(Ok(None), |provider| {
                provider.resolve_current_profile(context, identity)
            })
    }

    /// Proposes a semantic profile through the configured shared provider.
    ///
    /// # Parameters
    /// - `self`: Shared application facade.
    /// - `context`: Trusted caller capabilities.
    /// - `base`: Exact current profile snapshot used for the candidate.
    /// - `profile`: Desired provider-neutral profile descriptor.
    ///
    /// # Returns
    /// An immutable candidate bound to its base and candidate snapshots.
    ///
    /// # Errors
    /// Returns authorization, validation, stale-base, storage, or unsupported errors.
    pub fn propose_agent_profile(
        &self,
        context: &ApplicationContext,
        base: SnapshotId,
        profile: AgentProfileDescriptor,
    ) -> Result<ProfileCandidate, AgentProfileError> {
        self.agent_profile_provider
            .as_ref()
            .ok_or_else(|| AgentProfileError::Unsupported("profile proposal".to_owned()))?
            .propose_profile(context, base, profile)
    }

    /// Promotes one semantic profile candidate through shared root compare-and-swap.
    ///
    /// # Parameters
    /// - `self`: Shared application facade.
    /// - `context`: Trusted caller capabilities.
    /// - `candidate`: Exact candidate returned by `propose_agent_profile`.
    ///
    /// # Returns
    /// Whether promotion succeeded or the root moved concurrently.
    ///
    /// # Errors
    /// Returns authorization, ancestry, storage, or unsupported errors.
    pub fn promote_agent_profile_candidate(
        &self,
        context: &ApplicationContext,
        candidate: &ProfileCandidate,
    ) -> Result<AgentProfileStoreResult, AgentProfileError> {
        self.agent_profile_provider
            .as_ref()
            .ok_or_else(|| AgentProfileError::Unsupported("profile promotion".to_owned()))?
            .promote_candidate(context, candidate)
    }

    /// Creates a profile candidate from one exact legacy profile selection record.
    ///
    /// The caller supplies semantic profile content and the chosen active
    /// upstream pair. The legacy profile name must match the candidate's stable
    /// identity; no provider/model identity is inferred from the retired data.
    ///
    /// # Parameters
    /// - `self`: Shared application facade.
    /// - `context`: Trusted caller capabilities for session inspection and profile proposal.
    /// - `request`: Exact legacy record and selection revision, profile snapshot, and operator-selected model.
    ///
    /// # Returns
    /// An immutable profile candidate; the current profile root remains unchanged.
    ///
    /// # Errors
    /// Returns authorization, missing-record, catalog, validation, stale-base, storage, or configuration errors.
    pub fn propose_legacy_profile_mapping(
        &self,
        context: &ApplicationContext,
        mut request: LegacyProfileMappingRequest,
    ) -> Result<ProfileCandidate, ApplicationError>
    where
        B: crate::ApplicationBackend,
    {
        super::service_sessions::authorize_session_model_selection(context, request.session)?;
        if request.legacy_path != "/execution" {
            return Err(ApplicationError::InvalidRequest {
                operation: "legacy profile mapping".to_owned(),
                message: "profile recovery requires the /execution record".to_owned(),
            });
        }
        let store =
            self.session_store
                .as_ref()
                .ok_or_else(|| ApplicationError::UnsupportedOperation {
                    operation: "legacy profile mapping".to_owned(),
                })?;
        let session = store
            .load(request.session)
            .map_err(|message| ApplicationError::Internal { message })?
            .ok_or_else(|| ApplicationError::NotFound {
                subject: format!("session {}", request.session),
            })?;
        if session.execution.revision != request.expected_revision {
            return Err(ApplicationError::Conflict {
                subject: format!("session {} execution selection", request.session),
                details: Box::new(crate::ConflictDetails {
                    operation: Some(format!(
                        "stale selection revision {}; current revision is {}",
                        request.expected_revision, session.execution.revision
                    )),
                    ..crate::ConflictDetails::default()
                }),
            });
        }
        let records = store
            .legacy_execution_records(request.session)
            .map_err(|message| ApplicationError::Internal { message })?;
        let record = records
            .iter()
            .find(|record| record.path == request.legacy_path)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: format!("legacy execution record {}", request.legacy_path),
            })?;
        let legacy_identity = record
            .value
            .get("model_profile")
            .and_then(serde_json::Value::as_str)
            .filter(|identity| !identity.trim().is_empty())
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "legacy profile mapping".to_owned(),
                message: "the selected legacy record contains no profile identity".to_owned(),
            })?;
        if request.profile.reference.identity != legacy_identity {
            return Err(ApplicationError::InvalidRequest {
                operation: "legacy profile mapping".to_owned(),
                message: "the semantic profile identity must preserve the exact legacy identity"
                    .to_owned(),
            });
        }
        let (model, effective_thinking) = self
            .resolve_upstream_model(
                &request.provider_id,
                &request.model_id,
                request.thinking_level.as_deref(),
            )
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "legacy profile mapping".to_owned(),
                message: error.to_string(),
            })?;
        request.profile.preferred_execution = Some(AgentProfileExecutionPreference {
            provider_id: model.provider,
            model_id: model.id,
            thinking_level: request
                .thinking_level
                .as_ref()
                .map(|_| effective_thinking.as_str().to_owned()),
        });
        self.propose_agent_profile(context, request.base_profile_snapshot, request.profile)
            .map_err(profile_application_error)
    }
}

/// Converts a semantic profile failure into a shared application error.
///
/// # Parameters
/// - `error`: Profile authorization, identity, or storage failure.
///
/// # Returns
/// The corresponding transport-neutral application failure.
fn profile_application_error(error: AgentProfileError) -> ApplicationError {
    match error {
        AgentProfileError::Unauthorized { action, profile } => ApplicationError::PermissionDenied {
            action,
            scope: format!("profile {profile}"),
        },
        AgentProfileError::NotFound { profile, .. } => ApplicationError::NotFound {
            subject: format!("agent profile {profile}"),
        },
        AgentProfileError::Unsupported(operation) => {
            ApplicationError::UnsupportedOperation { operation }
        }
        AgentProfileError::Invalid(message) => ApplicationError::InvalidRequest {
            operation: "agent profile".to_owned(),
            message,
        },
        AgentProfileError::StaleBase { expected, actual } => match actual {
            Some(current) => ApplicationError::StaleBaseSnapshot {
                requested: expected,
                current,
            },
            None => ApplicationError::Conflict {
                subject: "agent profile root changed".to_owned(),
                details: Box::new(crate::ConflictDetails::default()),
            },
        },
        AgentProfileError::Store(message) | AgentProfileError::Encoding(message) => {
            ApplicationError::Internal { message }
        }
    }
}
