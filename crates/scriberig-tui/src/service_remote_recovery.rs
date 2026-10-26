//! Implements explicit recovery operations for the remote protocol adapter.

use super::RemoteAdapter;
use crate::service::service_remote_mapping::{remote_error, session_plan_from_response};
use crate::service::{ApplicationContext, TuiRecoveryService};
use scriberig_app::ApplicationError;
use scriberig_core::EntityId;
use scriberig_protocol::ProtocolTransport;

impl<T: ProtocolTransport> TuiRecoveryService for RemoteAdapter<T> {
    /// Reads sanitized legacy execution recovery records from the remote service.
    ///
    /// - `_context`: Local context; remote authorization is transport-owned.
    /// - `session`: Exact session identity to inspect.
    ///
    /// Preserved records with exact source paths and stable targets.
    ///
    /// # Returns
    /// The records retained for operator review.
    ///
    /// # Errors
    /// Returns a sanitized protocol or transport error.
    fn legacy_execution_records(
        &self,
        _context: &ApplicationContext,
        session: EntityId,
    ) -> Result<Vec<scriberig_protocol::LegacyExecutionRecordView>, ApplicationError> {
        self.client
            .legacy_execution_records(session.to_string())
            .map_err(remote_error)
    }

    /// Maps one legacy session choice through the remote application service.
    ///
    /// - `_context`: Local context; remote authorization is transport-owned.
    /// - `request`: Exact path, observed revision, and selected model.
    ///
    /// # Returns
    /// Unit after the remote service confirms the mapping.
    ///
    /// # Errors
    /// Returns a sanitized validation, conflict, protocol, or transport error.
    fn map_legacy_execution_selection(
        &self,
        _context: &ApplicationContext,
        request: &scriberig_app::LegacyExecutionMappingRequest,
    ) -> Result<(), ApplicationError> {
        self.client
            .map_legacy_execution_selection(
                request.session.to_string(),
                request.expected_revision,
                request.legacy_path.clone(),
                request.provider_id.clone(),
                request.model_id.clone(),
                request.thinking_level.clone(),
            )
            .map_err(remote_error)?;
        Ok(())
    }

    /// Maps one current plan assignment through the remote application service.
    ///
    /// - `_context`: Local context; remote authorization is transport-owned.
    /// - `request`: Exact path, observed plan revision, and selected model.
    ///
    /// # Returns
    /// The updated durable plan.
    ///
    /// # Errors
    /// Returns a sanitized validation, conflict, protocol, or transport error.
    fn map_legacy_plan_assignment(
        &self,
        _context: &ApplicationContext,
        request: &scriberig_app::LegacyPlanAssignmentMappingRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        let response = self
            .client
            .map_legacy_plan_assignment(
                request.session.to_string(),
                request.legacy_path.clone(),
                request.expected_plan_revision,
                request.provider_id.clone(),
                request.model_id.clone(),
                request.thinking_level.clone(),
            )
            .map_err(remote_error)?;
        session_plan_from_response(response)
    }
}
