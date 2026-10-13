//! Provides deterministic provider scripts for agent conformance tests.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use scriberig_agent_core::ToolInput;
use scriberig_agent_runtime::{AgentMessage, ModelDecision, ModelProvider};

/// One provider-neutral decision in a deterministic scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderStep {
    /// Return a terminal assistant response.
    Final(String),
    /// Request one bounded child branch.
    Child {
        /// Child purpose exposed to the trusted executor.
        purpose: String,
        /// Child prompt exposed to the trusted executor.
        prompt: String,
    },
    /// Request one provider-neutral atomic tool call.
    Tool(ToolInput),
    /// Fail the provider call with a safe category.
    Error(String),
}

/// Records provider-neutral messages while replaying a fixed script.
#[derive(Clone, Debug)]
pub struct ScriptedProvider {
    /// Decisions returned in invocation order.
    steps: VecDeque<ProviderStep>,
    /// Messages received by each provider invocation.
    observed: Arc<Mutex<Vec<Vec<AgentMessage>>>>,
}

impl ScriptedProvider {
    /// Creates a provider from a deterministic decision script.
    ///
    /// # Parameters
    /// - `steps`: Decisions returned in order.
    ///
    /// # Returns
    /// A provider and a shared observation log.
    #[must_use]
    pub fn new(
        steps: impl IntoIterator<Item = ProviderStep>,
    ) -> (Self, Arc<Mutex<Vec<Vec<AgentMessage>>>>) {
        let observed = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                steps: steps.into_iter().collect(),
                observed: Arc::clone(&observed),
            },
            observed,
        )
    }
}

impl ModelProvider for ScriptedProvider {
    /// Records the semantic context and returns the next scripted decision.
    ///
    /// # Parameters
    /// - `messages`: Provider-neutral branch context.
    ///
    /// # Returns
    /// The next scripted model decision.
    ///
    /// # Errors
    /// Returns the scripted error or an error when the script is exhausted.
    fn decide(&mut self, messages: &[AgentMessage]) -> Result<ModelDecision, String> {
        self.observed
            .lock()
            .map_err(|_| "provider observation lock failed".to_owned())?
            .push(messages.to_vec());
        match self
            .steps
            .pop_front()
            .ok_or_else(|| "provider script exhausted".to_owned())?
        {
            ProviderStep::Final(output) => Ok(ModelDecision::Final(output)),
            ProviderStep::Child { purpose, prompt } => Ok(ModelDecision::Child(
                scriberig_agent_runtime::ChildRequest {
                    purpose,
                    prompt,
                    capabilities: vec!["inspect.entity".to_owned()],
                    expected_result: "compact result".to_owned(),
                },
            )),
            ProviderStep::Tool(input) => Ok(ModelDecision::Tool(input)),
            ProviderStep::Error(error) => Err(error),
        }
    }
}
