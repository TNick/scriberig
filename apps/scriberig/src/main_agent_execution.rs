//! Composes the shared CLI invocation through the upstream agent loop.

#[path = "main_agent_execution/upstream.rs"]
mod upstream;

pub(crate) use upstream::{ComposedAgentFailure, execute_model};
