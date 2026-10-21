//! Provides a synchronous host bridge for branch invocation callers.

use super::{
    UpstreamBranchFailure, UpstreamBranchInvocation, UpstreamBranchResult,
    run_upstream_branch_invocation,
};

/// Runs an upstream branch invocation from a synchronous application boundary.
///
/// This is for product operation adapters that already run on a worker thread.
/// It creates a scoped current-thread executor and does not introduce a second
/// provider or tool loop.
///
/// # Parameters
/// - `invocation`: Exact provider, model, branch context, and approved tools.
/// - `prompt`: Current user instruction for this branch invocation.
///
/// # Returns
/// The final assistant text and original branch identity.
///
/// # Errors
/// Returns invocation failures or failure to create the async executor.
pub fn run_upstream_branch_invocation_blocking(
    invocation: UpstreamBranchInvocation,
    prompt: impl Into<String>,
) -> Result<UpstreamBranchResult, UpstreamBranchFailure> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|error| UpstreamBranchFailure {
            error: super::UpstreamBranchError::Agent(format!(
                "could not start invocation executor: {error}"
            )),
            model_requests: Vec::new(),
        })?;
    runtime.block_on(run_upstream_branch_invocation(
        invocation,
        prompt,
        tokio_util::sync::CancellationToken::new(),
        None,
    ))
}
