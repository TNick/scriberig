//! Implements preserved-choice recovery for the embedded application adapter.

use super::ApplicationOperationAdapter;
use crate::service::{ApplicationContext, TuiRecoveryService};
use scriberig_app::{ApplicationBackend, ApplicationError};
use scriberig_core::EntityId;

impl<B: ApplicationBackend> TuiRecoveryService for ApplicationOperationAdapter<B> {
    /// Reads sanitized legacy execution recovery records from the application.
    ///
    /// - `context`: Caller authorization context.
    /// - `session`: Exact session identity to inspect.
    ///
    /// # Returns
    /// The records retained for operator review.
    ///
    /// # Errors
    /// Returns an application authorization or storage error.
    fn legacy_execution_records(
        &self,
        context: &ApplicationContext,
        session: EntityId,
    ) -> Result<Vec<scriberig_protocol::LegacyExecutionRecordView>, ApplicationError> {
        self.service
            .legacy_execution_records(context, session)
            .map(|records| {
                records
                    .into_iter()
                    .map(|record| scriberig_protocol::LegacyExecutionRecordView {
                        path: record.path,
                        value: record.value,
                        target: record.target.map(|target| {
                            scriberig_protocol::LegacyExecutionTargetView {
                                plan_id: target.plan_id.clone(),
                                node_id: target.node_id.clone(),
                                revision: target.revision,
                                claim: target.claim,
                            }
                        }),
                    })
                    .collect()
            })
    }

    /// Maps one legacy session selection through active catalog validation.
    ///
    /// - `context`: Caller authorization context.
    /// - `request`: Exact recovery path, expected revision, and selected model.
    ///
    /// # Returns
    /// Unit after the shared application confirms the mapping.
    ///
    /// # Errors
    /// Returns a validation, revision-conflict, authorization, or storage error.
    fn map_legacy_execution_selection(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::LegacyExecutionMappingRequest,
    ) -> Result<(), ApplicationError> {
        self.service
            .map_legacy_execution_selection(context, request)
    }

    /// Maps one current plan assignment through active catalog validation.
    ///
    /// - `context`: Caller authorization context.
    /// - `request`: Exact recovery path, expected plan revision, and selected model.
    ///
    /// # Returns
    /// The updated durable plan.
    ///
    /// # Errors
    /// Returns a validation, revision-conflict, authorization, or storage error.
    fn map_legacy_plan_assignment(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::LegacyPlanAssignmentMappingRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        self.service.map_legacy_plan_assignment(context, request)
    }
}
