//! Implements session-scoped model-selection persistence helpers.

use super::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService, EntityId,
    Session, empty_session,
};
use crate::ConflictDetails;
use scriberig_session::{SelectionError, SelectionStoreError};

impl<B: ApplicationBackend> ApplicationService<B> {
    /// Resolves an exact provider/model selection through upstream machinery.
    ///
    /// - `self`: Application service owning the model library.
    /// - `provider_id`: Upstream provider identity.
    /// - `model_id`: Provider-scoped upstream model identity.
    /// - `thinking_level`: Optional canonical upstream thinking level.
    ///
    /// The exact upstream model and its effective thinking level.
    ///
    /// Returns not-found or invalid-request for unavailable configuration.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn resolve_upstream_model(
        &self,
        provider_id: &str,
        model_id: &str,
        thinking_level: Option<&str>,
    ) -> Result<(rpi_ai::Model, rpi_ai::ThinkingLevel), ApplicationError> {
        let catalog_snapshot = self.model_catalog.snapshot();
        let thinking = thinking_level
            .map(|level| crate::ThinkingLevelId::new(level.to_owned()).map_err(invalid_selection))
            .transpose()?
            .map_or(rpi_ai::ThinkingLevel::Off, |level| {
                rpi_ai::ThinkingLevel::parse_name(level.as_str())
                    .expect("ThinkingLevelId uses upstream parsing")
            });
        let resolved = rpi_model_runtime::resolve_selection(
            &catalog_snapshot.models,
            &rpi_model_runtime::ModelSelection {
                provider_id: provider_id.to_owned(),
                model_id: model_id.to_owned(),
                thinking,
            },
        )
        .map_err(|error| invalid_selection(error.to_string()))?;
        Ok((
            catalog_snapshot.models[resolved.catalog_index].clone(),
            resolved.effective_thinking,
        ))
    }

    /// Loads session state and identifies the subsystem that owns persistence.
    ///
    /// - `self`: Application service providing session storage.
    /// - `context`: Authorized request context for backend reads.
    /// - `session`: Stable session identity.
    ///
    /// The session and whether an application backend owns its persistence.
    ///
    /// Returns an application error for failed or unauthorized reads.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn selection_session(
        &self,
        context: &ApplicationContext,
        session: EntityId,
    ) -> Result<(Session, bool), ApplicationError> {
        if let Some(value) = self.load_session_state(session)? {
            return Ok((value, false));
        }
        match self
            .backend()
            .read_session(context, &crate::SessionRequest { session })
        {
            Ok(value) => Ok((value, true)),
            Err(error) if matches!(error.category(), "unsupported_operation" | "not_found") => {
                Ok((empty_session(session), false))
            }
            Err(error) => Err(error),
        }
    }

    /// Persists a selection update through its owning session boundary.
    ///
    /// - `self`: Application service providing session storage.
    /// - `context`: Authorized request context for backend writes.
    /// - `session`: Complete session with the updated execution selection.
    /// - `backend_owned`: Whether the backend supplied the current session.
    /// - `expected_revision`: Selection revision observed by the caller.
    ///
    /// Unit after selection persistence succeeds.
    ///
    /// Returns persistence errors or unsupported-operation for read-only state.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn persist_selection(
        &self,
        context: &ApplicationContext,
        session: Session,
        backend_owned: bool,
        expected_revision: u64,
    ) -> Result<(), ApplicationError> {
        if backend_owned {
            return self.backend().update_session_execution_selection(
                context,
                &session,
                expected_revision,
            );
        }
        if let Some(store) = &self.session_store {
            return store
                .compare_and_swap_execution_selection(&session, expected_revision)
                .map_err(|error| match error {
                    SelectionStoreError::RevisionConflict { expected, actual } => selection_error(
                        SelectionError::RevisionConflict { expected, actual },
                        session.id,
                    ),
                    SelectionStoreError::Persistence(message) => {
                        ApplicationError::Internal { message }
                    }
                });
        }
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| ApplicationError::Internal {
                message: "application session lock failed".to_owned(),
            })?;
        let actual_revision = sessions
            .get(&session.id)
            .map_or(0, |current| current.execution.revision);
        ensure_revision(expected_revision, actual_revision, session.id)?;
        if let Some(current) = sessions.get_mut(&session.id) {
            current.execution = session.execution;
        } else {
            sessions.insert(session.id, session);
        }
        Ok(())
    }
}

/// Rejects a stale session selection mutation before configuration is read.
///
/// - `expected`: Revision observed by the caller.
/// - `actual`: Current selection revision.
/// - `session`: Session being changed.
///
/// Unit when the observed revision is current.
///
/// Returns a structured conflict when revisions differ.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn ensure_revision(
    expected: u64,
    actual: u64,
    session: EntityId,
) -> Result<(), ApplicationError> {
    if expected == actual {
        return Ok(());
    }
    Err(selection_error(
        SelectionError::RevisionConflict { expected, actual },
        session,
    ))
}

/// Converts a selection-domain failure into a stable application error.
///
/// - `error`: Selection mutation failure.
/// - `session`: Session whose selection was targeted.
///
/// Structured conflict or invalid-request error.
/// # Returns
/// The operation result.
pub(super) fn selection_error(error: SelectionError, session: EntityId) -> ApplicationError {
    match error {
        SelectionError::RevisionConflict { expected, actual } => ApplicationError::Conflict {
            subject: format!("session {session} execution selection"),
            details: Box::new(ConflictDetails {
                operation: Some(format!(
                    "stale selection revision {expected}; current revision is {actual}"
                )),
                ..ConflictDetails::default()
            }),
        },
        SelectionError::InconsistentSelection => {
            invalid_selection("model profile and model must be selected or cleared together")
        }
        SelectionError::RevisionExhausted => ApplicationError::Internal {
            message: "session execution selection revision is exhausted".to_owned(),
        },
    }
}

/// Creates a safe invalid model-selection request error.
///
/// - `message`: Configuration diagnostic without provider credentials.
///
/// The application error for invalid selection input.
/// # Returns
/// The operation result.
pub(super) fn invalid_selection(message: impl Into<String>) -> ApplicationError {
    ApplicationError::InvalidRequest {
        operation: "session model selection".to_owned(),
        message: message.into(),
    }
}
