//! Provides unsupported model-control methods for adapter fixtures.

use super::*;

impl scriberig_app::AgentConsoleBackend for LocalSessionFixture {
    /// Rejects model-library reads in the session adapter fixture.
    ///
    /// # Parameters
    /// - `_context`: Application request context.
    ///
    /// # Returns
    /// Never returns a model library.
    ///
    /// # Errors
    /// Always returns an unsupported-operation error.
    fn agent_model_library(
        &self,
        _context: &ApplicationContext,
    ) -> Result<scriberig_app::ModelLibrary, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "fixture model library".to_owned(),
        })
    }

    /// Rejects model selection in the session adapter fixture.
    ///
    /// # Parameters
    /// - `_context`: Application request context.
    /// - `_request`: Requested session model selection.
    ///
    /// # Returns
    /// Never returns an updated selection.
    ///
    /// # Errors
    /// Always returns an unsupported-operation error.
    fn set_session_model(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::SessionModelSelectionRequest,
    ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "fixture model selection".to_owned(),
        })
    }

    /// Rejects thinking selection in the session adapter fixture.
    ///
    /// # Parameters
    /// - `_context`: Application request context.
    /// - `_request`: Requested session thinking selection.
    ///
    /// # Returns
    /// Never returns an updated selection.
    ///
    /// # Errors
    /// Always returns an unsupported-operation error.
    fn set_session_thinking_level(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::SessionThinkingSelectionRequest,
    ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "fixture thinking selection".to_owned(),
        })
    }
}
