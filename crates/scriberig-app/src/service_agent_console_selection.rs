//! Implements session-scoped model-selection persistence helpers.

use super::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService, EntityId,
    ModelDescriptor, ModelProfile, SelectionError, SelectionStoreError, Session, empty_session,
    ensure_revision, invalid_selection, selection_error,
};

impl<B: ApplicationBackend> ApplicationService<B> {
    /// Resolves an enabled profile and its model descriptor.
    ///
    /// # Parameters
    /// - `self`: Application service owning the model library.
    /// - `profile_id`: Stable model-profile identity.
    ///
    /// # Returns
    /// The profile and its validated model descriptor.
    ///
    /// # Errors
    /// Returns not-found or invalid-request for unavailable configuration.
    pub(super) fn resolve_profile(
        &self,
        profile_id: &str,
    ) -> Result<(&ModelProfile, &ModelDescriptor), ApplicationError> {
        let profile =
            self.model_library
                .profile(profile_id)
                .ok_or_else(|| ApplicationError::NotFound {
                    subject: format!("model profile {profile_id}"),
                })?;
        if !profile.enabled {
            return Err(invalid_selection("model profile is disabled"));
        }
        let model =
            self.model_library
                .model(&profile.model)
                .ok_or_else(|| ApplicationError::NotFound {
                    subject: format!("model {}", profile.model.as_str()),
                })?;
        profile
            .validate_for_model(model)
            .map_err(invalid_selection)?;
        Ok((profile, model))
    }

    /// Loads session state and identifies the subsystem that owns persistence.
    ///
    /// # Parameters
    /// - `self`: Application service providing session storage.
    /// - `context`: Authorized request context for backend reads.
    /// - `session`: Stable session identity.
    ///
    /// # Returns
    /// The session and whether an application backend owns its persistence.
    ///
    /// # Errors
    /// Returns an application error for failed or unauthorized reads.
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
    /// # Parameters
    /// - `self`: Application service providing session storage.
    /// - `context`: Authorized request context for backend writes.
    /// - `session`: Complete session with the updated execution selection.
    /// - `backend_owned`: Whether the backend supplied the current session.
    /// - `expected_revision`: Selection revision observed by the caller.
    ///
    /// # Returns
    /// Unit after selection persistence succeeds.
    ///
    /// # Errors
    /// Returns persistence errors or unsupported-operation for read-only state.
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
