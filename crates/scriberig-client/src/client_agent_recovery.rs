//! Provides protocol client methods for explicit legacy choice recovery.

use super::{Client, ClientError, ProtocolTransport, Request, Response, SessionReference};
use scriberig_protocol::{
    LegacyExecutionMappingRequest, LegacyExecutionRecordView, LegacyExecutionRecordsRequest,
    LegacyPlanAssignmentMappingRequest, SessionExecutionSelection, SessionPlan,
};

impl<T: ProtocolTransport> Client<T> {
    /// Reads preserved, sanitized legacy execution values for one session.
    ///
    /// - `session`: Stable session identity to inspect.
    ///
    /// Records requiring explicit operator mapping.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn legacy_execution_records(
        &self,
        session: impl Into<String>,
    ) -> Result<Vec<LegacyExecutionRecordView>, ClientError> {
        match self
            .send(Request::LegacyExecutionRecords(
                LegacyExecutionRecordsRequest {
                    session: SessionReference { id: session.into() },
                },
            ))?
            .response
        {
            Response::LegacyExecutionRecords(records) => Ok(records),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Maps one preserved legacy execution record to an active model selection.
    ///
    /// - `session`: Stable session identity.
    /// - `expected_revision`: Selection revision observed by the operator.
    /// - `legacy_path`: Exact path returned by `legacy_execution_records`.
    /// - `provider_id`: Active upstream provider identity.
    /// - `model_id`: Provider-scoped active model identity.
    /// - `thinking_level`: Optional active thinking-level identity.
    ///
    /// Updated session execution selection.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns validation, authorization, conflict, or persistence failures.
    pub fn map_legacy_execution_selection(
        &self,
        session: impl Into<String>,
        expected_revision: u64,
        legacy_path: impl Into<String>,
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        thinking_level: Option<String>,
    ) -> Result<SessionExecutionSelection, ClientError> {
        match self
            .send(Request::LegacyExecutionMapping(
                LegacyExecutionMappingRequest {
                    session: SessionReference { id: session.into() },
                    expected_revision,
                    legacy_path: legacy_path.into(),
                    provider_id: provider_id.into(),
                    model_id: model_id.into(),
                    thinking_level,
                },
            ))?
            .response
        {
            Response::SessionExecution(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Maps one current legacy plan assignment to an active upstream model.
    ///
    /// - `session`: Stable session identity.
    /// - `legacy_path`: Exact path returned by `legacy_execution_records`.
    /// - `expected_plan_revision`: Plan revision observed by the operator.
    /// - `provider_id`: Active upstream provider identity.
    /// - `model_id`: Provider-scoped active model identity.
    /// - `thinking_level`: Optional active thinking-level identity.
    ///
    /// Updated durable plan.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns validation, authorization, conflict, or persistence failures.
    pub fn map_legacy_plan_assignment(
        &self,
        session: impl Into<String>,
        legacy_path: impl Into<String>,
        expected_plan_revision: u64,
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        thinking_level: Option<String>,
    ) -> Result<SessionPlan, ClientError> {
        match self
            .send(Request::LegacyPlanAssignmentMapping(
                LegacyPlanAssignmentMappingRequest {
                    session: SessionReference { id: session.into() },
                    legacy_path: legacy_path.into(),
                    expected_plan_revision,
                    provider_id: provider_id.into(),
                    model_id: model_id.into(),
                    thinking_level,
                },
            ))?
            .response
        {
            Response::Plan(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
