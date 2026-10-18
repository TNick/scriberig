//! Configures runtime budgets on an agent execution loop.

use super::AgentRuntime;
use crate::OutputCallback;
use crate::orchestration::{RuntimeEventLog, RuntimeLimits};
use scriberig_app::OperationId;
use scriberig_app::OperationRegistry;

impl<P, T> AgentRuntime<P, T> {
    /// Publishes runtime activity under an already-running application operation.
    ///
    /// # Parameters
    /// - `self`: Runtime being configured.
    /// - `registry`: Shared operation event registry.
    /// - `operation`: Existing application operation identity.
    ///
    /// # Returns
    /// The runtime configured to publish transient activity without owning the
    /// operation lifecycle.
    #[must_use]
    pub fn with_observed_operation(
        mut self,
        registry: OperationRegistry,
        operation: OperationId,
    ) -> Self {
        self.registry = Some(registry);
        self.observed_operation = Some(operation);
        self
    }

    /// Attaches an application operation registry to this runtime.
    ///
    /// # Parameters
    /// - `self`: Runtime being configured.
    /// - `registry`: Shared lifecycle registry.
    ///
    /// # Returns
    /// The configured runtime.
    #[must_use]
    pub fn with_operation_registry(mut self, registry: OperationRegistry) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Attaches a bounded event log to this runtime.
    ///
    /// # Parameters
    /// - `self`: Runtime being configured.
    /// - `event_log`: Event destination for lifecycle observations.
    ///
    /// # Returns
    /// The runtime configured to emit safe phase events.
    #[must_use]
    pub fn with_event_log(mut self, event_log: RuntimeEventLog) -> Self {
        self.event_log = Some(event_log);
        self
    }

    /// Attaches a callback for bounded provider-neutral text fragments.
    ///
    /// # Parameters
    /// - `self`: Runtime being configured.
    /// - `callback`: Thread-safe presentation or observation callback.
    ///
    /// # Returns
    /// The runtime configured to forward text fragments.
    #[must_use]
    pub fn with_output_callback(mut self, callback: OutputCallback) -> Self {
        self.output_callback = Some(callback);
        self
    }

    /// Assigns a stable branch correlation identifier for emitted events.
    ///
    /// # Parameters
    /// - `self`: Runtime being configured.
    /// - `branch_id`: Safe branch correlation identifier.
    ///
    /// # Returns
    /// The runtime configured with the branch identifier.
    #[must_use]
    pub fn with_branch_id(mut self, branch_id: impl Into<String>) -> Self {
        self.branch_id = branch_id.into();
        self
    }

    /// Replaces the runtime budgets enforced by the execution loop.
    ///
    /// # Parameters
    /// - `self`: Runtime being configured.
    /// - `limits`: Maximum work permitted for each branch.
    /// - `super`: Runtime module visibility context.
    ///
    /// # Returns
    /// The runtime with explicit budgets.
    #[must_use]
    pub fn with_limits(mut self, limits: RuntimeLimits) -> Self {
        self.limits = limits;
        self
    }
}
