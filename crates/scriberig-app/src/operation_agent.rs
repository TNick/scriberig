//! Publishes provider-neutral agent observations through the operation registry.

use super::{ApplicationError, ApplicationEvent, OperationId, OperationRegistry};

/// Bounds one transient activity metadata value by Unicode scalar values.
///
/// # Parameters
/// - `value`: Metadata value received from the trusted runtime boundary.
/// - `maximum`: Maximum number of Unicode scalar values to retain.
///
/// # Returns
/// The original value when within the limit, otherwise a bounded prefix.
fn bounded_value(value: &str, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}

impl OperationRegistry {
    /// Publishes bounded, transient agent status activity.
    ///
    /// # Parameters
    /// - `id`: Active agent operation producing the activity.
    /// - `branch`: Stable branch identity associated with the activity.
    /// - `activity_kind`: Provider-neutral activity category.
    /// - `state`: Lifecycle state for the activity.
    /// - `summary`: Safe concise status text.
    ///
    /// # Returns
    /// Success after publication to the monotonic operation event stream.
    ///
    /// # Errors
    /// Returns a structured failure when the operation is unknown or terminal.
    pub fn publish_agent_activity(
        &self,
        id: &OperationId,
        branch: impl Into<String>,
        activity_kind: scriberig_session::AgentActivityKind,
        state: scriberig_session::AgentActivityState,
        summary: impl Into<String>,
    ) -> Result<(), ApplicationError> {
        let mut state_guard = self.lock()?;
        let record = state_guard.record_mut(id)?;
        if record.status.is_terminal() {
            return Err(ApplicationError::OperationFailed {
                operation: id.to_string(),
                message: "terminal operation cannot publish activity".to_owned(),
            });
        }
        let branch = bounded_value(&branch.into(), 128);
        let summary = bounded_value(&summary.into(), 256);
        state_guard.publish(&ApplicationEvent::AgentActivity {
            operation: id.clone(),
            branch,
            activity_kind,
            state,
            summary,
        });
        Ok(())
    }

    /// Publishes a bounded transient agent output fragment.
    ///
    /// # Parameters
    /// - `id`: Active agent operation producing the fragment.
    /// - `text`: Incremental provider-neutral assistant text.
    ///
    /// # Returns
    /// Success after the event has been published.
    ///
    /// # Errors
    /// Returns a structured failure when the operation is unknown or terminal.
    pub fn publish_agent_output_delta(
        &self,
        id: &OperationId,
        text: impl Into<String>,
    ) -> Result<(), ApplicationError> {
        let mut state = self.lock()?;
        let record = state.record_mut(id)?;
        if record.status.is_terminal() {
            return Err(ApplicationError::OperationFailed {
                operation: id.to_string(),
                message: "terminal operation cannot publish output".to_owned(),
            });
        }
        let mut text = text.into();
        text.truncate(4096);
        state.publish(&ApplicationEvent::AgentOutputDelta {
            operation: id.clone(),
            text,
        });
        Ok(())
    }

    /// Publishes bounded agent output correlated to its active operation.
    ///
    /// # Parameters
    /// - `id`: Active agent operation producing output.
    /// - `output`: Bounded output text for transient presentation.
    ///
    /// # Returns
    /// Success after the event has been published.
    ///
    /// # Errors
    /// Returns a structured failure when the operation is unknown or terminal.
    pub fn publish_agent_output(
        &self,
        id: &OperationId,
        output: impl Into<String>,
    ) -> Result<(), ApplicationError> {
        let mut state = self.lock()?;
        let record = state.record_mut(id)?;
        if record.status.is_terminal() {
            return Err(ApplicationError::OperationFailed {
                operation: id.to_string(),
                message: "terminal operation cannot publish output".to_owned(),
            });
        }
        let mut output = output.into();
        output.truncate(4096);
        state.publish(&ApplicationEvent::AgentOutputProduced {
            operation: id.clone(),
            output,
        });
        Ok(())
    }
}
