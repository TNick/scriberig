//! Connects application session state to an optional durable repository.

use scriberig_core::EntityId;
use scriberig_session::Session;

use crate::{ApplicationError, ApplicationService};

impl<B> ApplicationService<B> {
    /// Loads a compact session read projection from durable or local state.
    ///
    /// - `self`: Application service owning session persistence.
    /// - `session_id`: Stable session identity to load.
    ///
    /// The stored projection when the session exists.
    ///
    /// # Returns
    /// The projection, or `None` when no session exists.
    ///
    /// # Errors
    /// Returns a repository or lock failure.
    pub(super) fn load_session_read_projection_state(
        &self,
        session_id: EntityId,
    ) -> Result<Option<Session>, ApplicationError> {
        if let Some(store) = &self.session_store {
            return store
                .load_read_projection(session_id)
                .map_err(|message| ApplicationError::Internal { message });
        }
        self.load_session_state(session_id)
    }

    /// Loads one session from durable storage or the process-local fallback.
    ///
    /// - `super`: Application service.
    /// - `self`: Application service.
    /// - `session_id`: Session identity to load.
    ///
    /// The stored session when it exists.
    ///
    /// Returns a repository or lock failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `super`: Application service.
    /// - `self`: Application service.
    /// - `session`: Complete session graph to persist.
    ///
    /// Unit after the graph is stored.
    ///
    /// Returns a repository or lock failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn save_session_state(&self, mut session: Session) -> Result<(), ApplicationError> {
        session.compact_branch_results();
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
