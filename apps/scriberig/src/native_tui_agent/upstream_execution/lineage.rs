//! Validates continuation lineage against the compiled semantic context.

use scriberig_app::{ApplicationBackend, ApplicationContext, ApplicationService};
use scriberig_core::EntityId;

use super::{UpstreamBranchExecutionFailure, UpstreamBranchFailure};

/// Recompiles a completed root branch's next provider context.
///
/// # Parameters
/// - `service`: Shared application service owning the durable session.
/// - `context`: Trusted root request context.
/// - `session`: Canonical `ScribeRig` session identity.
/// - `branch`: Canonical root branch identity.
/// - `prompt`: Completed instruction used to select the branch turn.
/// - `selection`: Exact selected workspace scope, when present.
///
/// # Returns
/// A fingerprint of the durable post-completion provider context.
///
/// # Errors
/// Returns a captured context-compilation failure.
pub(crate) fn completed_root_context_fingerprint<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
    branch: EntityId,
    prompt: &str,
    selection: Option<&scriberig_app::AgentSelection>,
) -> Result<String, UpstreamBranchExecutionFailure> {
    let semantic_scope =
        selection.map(
            |selection| scriberig_agent_runtime::context::ProjectSemanticScope {
                project: selection
                    .project
                    .map_or_else(|| "unknown".to_owned(), |project| project.to_string()),
                snapshot: selection.snapshot.to_string(),
                root: selection.root.to_string(),
                selection: Some(selection.entity.to_string()),
            },
        );
    let runtime_context = selection.map_or_else(
        || context.clone(),
        |selection| context.clone().with_snapshot(selection.snapshot),
    );
    let compiled = crate::agent_context::compile_branch_context(
        service,
        &runtime_context,
        session,
        branch,
        prompt,
        semantic_scope,
    )
    .map_err(|_| UpstreamBranchExecutionFailure {
        failure: UpstreamBranchFailure {
            error: scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Agent(
                "branch context compilation failed".to_owned(),
            ),
            model_requests: Vec::new(),
        },
        tool_evidence: Vec::new(),
    })?;
    Ok(provider_context_fingerprint(
        &compiled,
        &runtime_context.capabilities,
    ))
}

/// Fingerprints the provider context with a versioned domain separator.
///
/// - `compiled`: Complete context, including exact dependency identities.
/// - `capabilities`: Exact semantic grants for the invocation.
///
/// # Returns
/// A lineage fingerprint for branch-local continuation validation.
pub(super) fn provider_context_fingerprint(
    compiled: &scriberig_agent_runtime::context::CompiledContext,
    capabilities: &scriberig_app::CapabilityContext,
) -> String {
    let mut capability_rows = capabilities
        .granted
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    capability_rows.sort();
    let mut grants = capabilities
        .grants
        .iter()
        .map(|grant| format!("{grant:?}"))
        .collect::<Vec<_>>();
    grants.sort();
    let capability_identity = format!("{}\n{}", capability_rows.join("\n"), grants.join("\n"));
    let context_identity = format!(
        "{:?}\n{:?}\n{}\n{}\n{}",
        compiled.profile,
        compiled.active_components,
        compiled.dependencies.join("\n"),
        compiled.omitted_handles.join("\n"),
        compiled.provider_context,
    );
    scriberig_agent_runtime::context::RequestFingerprintInput::new(
        "provider-continuation-v1",
        "",
        format!("{context_identity}\n{capability_identity}"),
        "",
    )
    .fingerprint()
}
