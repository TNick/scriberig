//! Configures runtime budgets on an agent execution loop.

use super::AgentRuntime;
use crate::orchestration::RuntimeLimits;

impl<P, T> AgentRuntime<P, T> {
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
