//! Admits configured isolated plugin tools into one authorized branch.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::{UpstreamBranchExecutionFailure, execution};
use scriberig_agent_runtime::plugin_worker::{
    ApplicationDomainDispatcher, PluginToolRequest, PluginWorkerAdmission,
    PluginWorkerConfiguration, WorkerSandboxPolicy, compose_branch_plugin_tools,
    sandboxed_launcher,
};
use scriberig_app::{ApplicationBackend, ApplicationContext, ApplicationService};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;

/// Extends semantic branch tools with configured isolated worker adapters.
///
/// # Parameters
/// - `tools`: Already authorized `ScribeRig` semantic tool adapters.
/// - `service`: Shared application boundary for worker host calls.
/// - `context`: Immutable branch context and capability snapshot.
/// - `branch`: Canonical `ScribeRig` branch identity.
/// - `project_store`: Durable store whose database must remain inaccessible.
/// - `invocation`: Branch activity sequence and provider execution context.
/// - `evidence`: Shared accumulator for settled branch tool evidence.
///
/// # Returns
/// Semantic and plugin adapters for this branch.
///
/// # Errors
/// Returns an execution failure when configured worker admission fails.
pub(super) fn append_plugin_tools<B>(
    mut tools: Vec<Arc<scriberig_agent_runtime::upstream_tool::SemanticAgentTool>>,
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    branch: EntityId,
    project_store: &Arc<SqliteStore>,
    invocation: &scriberig_agent_runtime::upstream_invocation::UpstreamBranchInvocation,
    evidence: &Arc<Mutex<Vec<scriberig_app::ToolExecutionEvidence>>>,
) -> Result<
    Vec<Arc<scriberig_agent_runtime::upstream_tool::SemanticAgentTool>>,
    UpstreamBranchExecutionFailure,
>
where
    B: ApplicationBackend + Send + Sync + 'static,
{
    tools.extend(compose_plugin_tools(
        service,
        context,
        branch,
        project_store,
        invocation,
        evidence,
    )?);
    Ok(tools)
}

/// Composes configured plugin tools for one branch after capability admission.
///
/// The package roots are selected by the host process, resolved through the
/// upstream project registry, and canonicalized by `PluginWorkerConfiguration`.
/// No worker starts unless this branch carries the configured plugin grant and
/// the runtime-verified platform launcher is available.
///
/// # Parameters
/// - `service`: Shared application boundary used for host semantic requests.
/// - `context`: Immutable branch capabilities and exact execution context.
/// - `branch`: Canonical branch identity owning the plugin adapters.
/// - `project_store`: Durable store whose database must remain inaccessible.
/// - `invocation`: Exact branch activity sequence and upstream context.
/// - `evidence`: Branch evidence accumulator shared with semantic dispatch.
///
/// # Returns
/// Adapters for tools authorized by both host allowlists and branch grants.
///
/// # Errors
/// Returns an execution failure when configured worker admission fails.
pub(super) fn compose_plugin_tools<B>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    branch: EntityId,
    project_store: &Arc<SqliteStore>,
    invocation: &scriberig_agent_runtime::upstream_invocation::UpstreamBranchInvocation,
    evidence: &Arc<Mutex<Vec<scriberig_app::ToolExecutionEvidence>>>,
) -> Result<
    Vec<Arc<scriberig_agent_runtime::upstream_tool::SemanticAgentTool>>,
    UpstreamBranchExecutionFailure,
>
where
    B: ApplicationBackend + Send + Sync + 'static,
{
    let Some(configuration) = tool_worker_configuration()? else {
        return Ok(Vec::new());
    };
    let capability = std::env::var("SCRIBE_AGENT_PLUGIN_CAPABILITY").map_err(|_| {
        execution::failure(
            scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Agent(
                "configured plugins require an explicit branch capability name".to_owned(),
            ),
            Vec::new(),
        )
    })?;
    let capability_id = scriberig_app::CapabilityId::new(capability.clone()).map_err(|_| {
        execution::failure(
            scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Agent(
                "configured plugin capability name is invalid".to_owned(),
            ),
            Vec::new(),
        )
    })?;
    if !context.capabilities.allows(&capability_id) {
        return Ok(Vec::new());
    }

    let launcher = sandboxed_launcher();
    if launcher.is_none() {
        return Err(execution::failure(
            scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Agent(
                "no runtime-verified isolated plugin launcher is available".to_owned(),
            ),
            Vec::new(),
        ));
    }
    let mut read_roots = plugin_read_roots(&configuration.launch.executable);
    read_roots.extend(configuration.launch.environment.values().flat_map(|value| {
        std::env::split_paths(std::ffi::OsStr::new(value))
            .filter(|path| !path.as_os_str().is_empty())
            .map(|path| {
                if path.is_file() {
                    path.parent()
                        .map_or(path.clone(), std::path::Path::to_path_buf)
                } else {
                    path
                }
            })
            .collect::<Vec<_>>()
    }));
    read_roots = canonical_roots(read_roots).map_err(|error| {
        execution::failure(
            scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Agent(error),
            Vec::new(),
        )
    })?;
    super::include_runtime_loader_aliases(&mut read_roots);
    let protected_roots = protected_roots(project_store).map_err(|error| {
        execution::failure(
            scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Agent(error),
            Vec::new(),
        )
    })?;
    let dispatcher =
        ApplicationDomainDispatcher::new(Arc::new(service.clone()), context.clone(), None);
    let dispatcher = match service.extension_component_mailbox() {
        Some(mailbox) => dispatcher.with_extension_components(mailbox),
        None => dispatcher,
    };
    let dispatcher = Arc::new(dispatcher);
    let evidence_sink = Arc::clone(evidence);
    let request = PluginToolRequest {
        catalog: crate::child_runtime::inspect_catalog(),
        capabilities: context.capabilities.clone(),
        branch,
        activity: Arc::clone(&invocation.activity),
        evidence: Arc::new(move |record| {
            if let Ok(mut records) = evidence_sink.lock() {
                records.push(record);
            }
        }),
    };
    let composed = compose_branch_plugin_tools(
        PluginWorkerAdmission {
            launcher,
            configuration,
            dispatcher,
            sandbox: WorkerSandboxPolicy::sandboxed(read_roots, Vec::new()),
            protected_roots,
            capability,
        },
        request,
    )
    .map_err(|error| {
        execution::failure(
            scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Agent(
                error.to_string(),
            ),
            Vec::new(),
        )
    })?;
    Ok(composed.tools().to_vec())
}

/// Loads plugin configuration only when the branch may expose plugin tools.
///
/// Provider-only packages are activated through the paired provider/catalog
/// generation and do not need a second worker for branch tool composition.
///
/// # Returns
/// A configured worker when at least one tool or operation is allowlisted.
///
/// # Errors
/// Returns an execution failure when the plugin environment is malformed.
fn tool_worker_configuration()
-> Result<Option<PluginWorkerConfiguration>, UpstreamBranchExecutionFailure> {
    let configuration = PluginWorkerConfiguration::from_environment().map_err(|error| {
        execution::failure(
            scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Agent(error),
            Vec::new(),
        )
    })?;
    Ok(configuration.filter(|configuration| {
        !configuration.allowed_tools.is_empty() || !configuration.allowed_operations.is_empty()
    }))
}

/// Builds read-only roots required by the worker and its selected extensions.
///
/// # Parameters
/// - `executable`: Explicit worker executable selected by configuration.
///
/// # Returns
/// Existing system, executable, and extension roots.
pub(crate) fn plugin_read_roots(executable: &std::path::Path) -> Vec<PathBuf> {
    let mut roots = ["/usr", "/bin", "/lib", "/lib64"]
        .into_iter()
        .map(PathBuf::from)
        .filter(|root| root.exists())
        .collect::<Vec<_>>();
    if let Some(parent) = executable.parent() {
        roots.push(parent.to_path_buf());
    }
    roots
}

/// Canonicalizes worker roots and rejects missing paths.
///
/// # Parameters
/// - `roots`: Read roots selected for this worker.
///
/// # Returns
/// Sorted and deduplicated canonical paths.
///
/// # Errors
/// Returns a safe error when a requested root cannot be resolved.
pub(crate) fn canonical_roots(roots: Vec<PathBuf>) -> Result<Vec<PathBuf>, String> {
    let mut canonical = roots
        .into_iter()
        .map(|root| root.canonicalize().map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    canonical.sort();
    canonical.dedup();
    Ok(canonical)
}

/// Adds symlink aliases used by the executable's dynamic loader.
///
/// Canonical path validation collapses `/lib` and `/lib64` on common Linux
/// distributions, but the ELF interpreter may still open those exact paths.
/// The aliases are added only after canonical containment checks have passed.
///
/// # Parameters
/// - `roots`: Validated canonical worker read roots to extend.
pub(crate) fn include_runtime_loader_aliases(roots: &mut Vec<PathBuf>) {
    roots.extend(
        ["/lib", "/lib64"]
            .into_iter()
            .map(PathBuf::from)
            .filter(|root| root.exists()),
    );
}

/// Collects database and secret registry paths the worker must not access.
///
/// # Parameters
/// - `project_store`: Durable application store.
///
/// # Returns
/// Canonical protected paths that currently exist.
///
/// # Errors
/// Returns a safe error when protected path metadata cannot be read.
pub(crate) fn protected_roots(project_store: &SqliteStore) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    if let Some(database) = project_store
        .database_path()
        .map_err(|_| "could not resolve the application database path".to_owned())?
    {
        roots.push(database);
    }
    let locations = scriberig_secrets::registry::RegistryLocations::from_process()
        .map_err(|_| "could not resolve the protected credential registry".to_owned())?;
    roots.push(locations.user_dir);
    if let Some(policy) = locations.system_policy {
        roots.push(policy);
    }
    roots
        .into_iter()
        .try_fold(Vec::new(), |mut resolved, path| {
            match path.canonicalize() {
                Ok(path) => resolved.push(path),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err("could not resolve protected application state".to_owned()),
            }
            Ok(resolved)
        })
}
