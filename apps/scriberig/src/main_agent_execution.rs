//! Composes the shared CLI invocation through the upstream agent loop.

#[path = "main_agent_execution/accepted_handle.rs"]
mod accepted_handle;
#[path = "main_agent_execution/upstream.rs"]
mod upstream;

pub(crate) use accepted_handle::from_protocol_acceptance;
pub(crate) use upstream::{ComposedAgentFailure, execute_model};
