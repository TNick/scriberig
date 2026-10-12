//! Connects application session state to an optional durable repository.

use scriberig_core::EntityId;
use scriberig_session::Session;

use crate::{ApplicationError, ApplicationService};

impl<B> ApplicationService<B> {
    /// Loads one session from durable storage or the process-local fallback.
    ///
    /// # Parameters
    /// - `super`: Application service.
    /// - `self`: Application service.
    /// - `session_id`: Session identity to load.
    ///
    /// # Returns
    /// The stored session when it exists.
    ///
    /// # Errors
    /// Returns a repository or lock failure.
    pub(super) fn load_session_state(
        &self,
        session_id: EntityId,
    ) -> Result<Option<Session>, ApplicationError> {
        if let Some(store) = &self.session_store {
            return store
                .load(session_id)
                .map_err(|message| ApplicationError::Internal { message });
        }
        self.sessions
            .lock()
            .map_err(|_| ApplicationError::Internal {
                message: "application session lock failed".to_owned(),
            })
            .map(|sessions| sessions.get(&session_id).cloned())
    }

    /// Persists one session and refreshes the process-local read cache.
    ///
    /// # Parameters
    /// - `super`: Application service.
    /// - `self`: Application service.
    /// - `session`: Complete session graph to persist.
    ///
    /// # Returns
    /// Unit after the graph is stored.
    ///
    /// # Errors
    /// Returns a repository or lock failure.
    pub(super) fn save_session_state(&self, session: Session) -> Result<(), ApplicationError> {
        if let Some(store) = &self.session_store {
            store
                .save(&session)
                .map_err(|message| ApplicationError::Internal { message })?;
        }
        self.sessions
            .lock()
            .map_err(|_| ApplicationError::Internal {
                message: "application session lock failed".to_owned(),
            })?
            .insert(session.id, session);
        Ok(())
    }
}
