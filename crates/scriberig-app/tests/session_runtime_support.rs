//! Provides shared session-runtime fixtures for integration tests.

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, CapabilityContext, CapabilityId,
};

/// Backend fixture that leaves session and agent ownership to the application.
pub struct Fixture;

impl ApplicationBackend for Fixture {
    /// Rejects universal requests because these tests target session fallback.
    ///
    /// # Parameters
    /// - `_context`: Application context, unused by the fixture.
    /// - `_request`: Universal request, intentionally unsupported.
    ///
    /// # Returns
    /// Never returns a universal result.
    ///
    /// # Errors
    /// Always returns unsupported-operation.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "fixture universal request".to_owned(),
        })
    }
}

/// Creates the session-tree capability used by authorized test callers.
///
/// # Returns
/// Application context granting the session and agent capabilities.
///
/// # Panics
/// Panics when a static fixture capability id is invalid.
#[must_use]
pub fn session_context() -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("capability"),
            CapabilityId::new("agent_start").expect("capability"),
            CapabilityId::new("agent_continue").expect("capability"),
            CapabilityId::new("agent_model_library").expect("capability"),
            CapabilityId::new("session_model_selection").expect("capability"),
        ]),
        ..ApplicationContext::empty()
    }
}
