//! Provides shared session-runtime fixtures for integration tests.

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, CapabilityContext, CapabilityId,
};
use scriberig_session::{ModelRequestMessage, ModelRequestProjection};

/// Backend fixture that leaves session and agent ownership to the application.
pub struct Fixture;

impl ApplicationBackend for Fixture {
    /// Rejects universal requests because these tests target session fallback.
    ///
    /// - `_context`: Application context, unused by the fixture.
    /// - `_request`: Universal request, intentionally unsupported.
    ///
    /// Never returns a universal result.
    ///
    /// Always returns unsupported-operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
/// Application context granting the session and agent capabilities.
///
/// # Panics
/// Panics when a static fixture capability id is invalid.
#[must_use]
/// # Returns
/// The operation result.
pub fn session_context() -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("capability"),
            CapabilityId::new("agent_start").expect("capability"),
            CapabilityId::new("agent_continue").expect("capability"),
            CapabilityId::new("agent_plan").expect("capability"),
            CapabilityId::new("agent_model_library").expect("capability"),
            CapabilityId::new("session_model_selection").expect("capability"),
        ]),
        ..ApplicationContext::empty()
    }
}

#[allow(
    dead_code,
    reason = "shared support module is compiled by integration targets with different fixtures"
)]
#[must_use]
/// Creates safe request evidence for a completed child branch.
///
/// # Returns
/// A bounded projection containing one safe user message.
pub fn child_model_request() -> ModelRequestProjection {
    ModelRequestProjection {
        ordinal: 0,
        model: "child-model".to_owned(),
        provider_id: None,
        provider_profile: None,
        thinking_level: None,
        profile: None,
        active_components: Vec::new(),
        context_fingerprint: None,
        request_fingerprint: None,
        catalog_revision: None,
        upstream_runtime_revision: None,
        provider_response_id: None,
        model_execution_configuration: None,
        messages: vec![ModelRequestMessage {
            role: "user".to_owned(),
            content: "safe request".to_owned(),
        }],
        full_messages: None,
        tools: Vec::new(),
        full_tools: None,
        content_complete: false,
        applied_settings: None,
        applied_settings_fingerprint: None,
        truncated: false,
    }
}
