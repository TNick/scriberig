//! Publishes provider-neutral agent observations through the operation registry.

use super::{ApplicationError, ApplicationEvent, OperationId, OperationRegistry};

impl OperationRegistry {
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
