//! Composes the integrated Scriberig executable from reusable and product runtime crates.

use std::collections::BTreeSet;
use std::sync::Arc;

use scriberig_agent_runtime::ChildExecutionScope;
use scriberig_app::{
    ApplicationBackendComposition, ApplicationContext, ApplicationService, Capability,
    CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityScope,
    GuardedQueryService, StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_store_sqlite::SqliteStore;

mod agent_config;
mod agent_context;
mod agent_tool;
mod agent_tool_helpers;
mod bootstrap_failure_provider;
mod child_runtime;
mod deterministic_provider;
mod doctor_command;
mod main_agent_execution;
mod native_scenario;
mod native_tui_agent;
mod remote_commands;
mod scenario_helpers;
mod secrets_command;
mod self_hosting_provider;
mod tui_command;
use main_agent_execution::execute_model;

/// Start the integrated `srig` executable and report runtime failures.
///
/// Unit after the selected command completes.
///
/// Returns an error when the API server cannot start or serve a request.
#[allow(clippy::too_many_lines)]
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let command = scriberig_cli::parse();
    match command {
        scriberig_cli::Command::Serve(args) => {
            let store = Arc::new(SqliteStore::open(&args.db)?);
            let runner = Arc::new(native_tui_agent::NativeTuiAgentRunner::from_environment(
                Arc::clone(&store),
            )?);
            let model_catalog = runner.model_catalog();
            let readiness_runner = Arc::clone(&runner);
            let provider_readiness_handler: Arc<scriberig_server::ProviderReadinessHandler> =
                Arc::new(move || {
                    scriberig_tui::AgentCommandRunner::<
                        scriberig_server::LocalApplicationBackend,
                    >::provider_readiness(readiness_runner.as_ref())
                });
            let plan_lifecycle_handler: Arc<scriberig_server::PlanLifecycleHandler> =
                Arc::new(move |service, context, request, action| {
                    let mut runtime_context = context.clone();
                    runtime_context.principal =
                        Some(scriberig_app::Principal::new("local-http-agent"));
                    runtime_context.capabilities.grants.push(Capability {
                        scope: CapabilityScope::Project,
                        actions: agent_actions(),
                        constraints: CapabilityConstraints::unrestricted(),
                    });
                    scriberig_tui::AgentCommandRunner::plan_lifecycle(
                        runner.as_ref(),
                        service,
                        &runtime_context,
                        request,
                        action,
                    )
                });
            scriberig_server::serve_with_model_catalog_and_plan_lifecycle(
                &args.host,
                args.port,
                &args.db,
                args.allow_origin.as_deref(),
                &args.project_roots,
                Some(plan_lifecycle_handler),
                scriberig_server::ServerModelCatalog {
                    model_catalog,
                    provider_readiness_handler: Some(provider_readiness_handler),
                },
            )?;
        }
        scriberig_cli::Command::Tui(args) => tui_command::run(&args)?,
        scriberig_cli::Command::Run(args) | scriberig_cli::Command::Agent(args) => {
            run_shared_agent(&args)?;
        }
        scriberig_cli::Command::Maintenance(args) => {
            use scriberig_store::ProtectedRoots;
            let store = SqliteStore::open(&args.db)?;
            let report = store.integrity_check(&ProtectedRoots::default())?;
            println!(
                "maintenance: {} corruption issue(s), {} unreachable snapshot(s)",
                report.corrupt.len(),
                report.unreachable.snapshots.len()
            );
            if report.is_corrupt() {
                return Err("maintenance integrity check found corruption".into());
            }
        }
        scriberig_cli::Command::Inspect(args) => {
            let store = SqliteStore::open(&args.db)?;
            let service = ApplicationService::new(StoreInspectionBackend::new(store));
            let service = GuardedQueryService::new(service);
            let context = ApplicationContext {
                capabilities: CapabilityContext::with_grants([Capability {
                    scope: CapabilityScope::Project,
                    actions: [CapabilityAction::Inspect]
                        .into_iter()
                        .collect::<BTreeSet<_>>(),
                    constraints: CapabilityConstraints::unrestricted(),
                }]),
                principal: Some(scriberig_app::Principal::new("local-cli")),
                ..ApplicationContext::empty()
            };
            let result =
                scriberig_cli::dispatch(&service, &context, scriberig_cli::Command::Inspect(args));
            match result {
                Ok(Some(output)) => println!("{output}"),
                Ok(None) => {}
                Err(error) => {
                    eprintln!("{}", error.message);
                    return Err(format!("inspect failed with status {}", error.exit_code).into());
                }
            }
        }
        scriberig_cli::Command::Capabilities(args) => {
            remote_commands::capabilities(&args)?;
        }
        scriberig_cli::Command::Operation(args) => {
            remote_commands::operation(args)?;
        }
        scriberig_cli::Command::Events(args) => {
            remote_commands::events(args)?;
        }
        scriberig_cli::Command::Session(args) => remote_commands::session(args)?,
        scriberig_cli::Command::AgentContinue(args) => {
            remote_commands::agent_continue(args)?;
        }
        scriberig_cli::Command::Project(args) => {
            scriberig_cli::project::run(&args)?;
        }
        scriberig_cli::Command::Config(args) => {
            agent_config::config_command(&args)?;
        }
        scriberig_cli::Command::Secrets(args) => {
            if let Err(error) = secrets_command::run(&args) {
                eprintln!("secrets: {error}");
                std::process::exit(error.exit_code());
            }
        }
        scriberig_cli::Command::Doctor(args) => {
            if !doctor_command::run(&args) {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

/// Executes a headless prompt through the shared application agent boundary.
///
/// - `args`: Parsed prompt, database, and output settings.
///
/// Unit after the terminal operation result is rendered.
///
/// Returns an error when the local store, session operation, or output fails.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn run_shared_agent(
    args: &scriberig_cli::RunArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = agent_config::resolve_run_config(args)?;
    let store = Arc::new(SqliteStore::open(&args.db)?);
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    );
    let session_store = Arc::new(scriberig_session_sqlite::SqliteSessionStore::open(
        &args.db,
    )?);
    let service = ApplicationService::new(backend).with_session_store(session_store);
    let session = fresh_session_id()?;
    let context = ApplicationContext {
        capabilities: scriberig_app::CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions: agent_actions(),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        principal: Some(scriberig_app::Principal::new("local-agent")),
        ..ApplicationContext::empty()
    };
    let request = scriberig_app::AgentCommandRequest {
        session,
        branch: None,
        anchor: None,
        prompt: args.prompt.clone(),
        selection: None,
    };
    let handle = service.start_local_agent(&request)?;
    let cancellation = service.operations().cancellation_flag(&handle.operation)?;
    let cancellation_handler = Arc::clone(&cancellation);
    ctrlc::set_handler(move || {
        cancellation_handler.store(true, std::sync::atomic::Ordering::Release);
    })?;
    let scope = ChildExecutionScope {
        session,
        parent_branch: handle.root_branch,
        depth: 0,
        active_branches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        semantic_scope: None,
    };
    let runtime_result =
        match run_composed_model(&args, &service, &store, &context, &scope, &cancellation) {
            Ok(result) => result,
            Err(error) => {
                let main_agent_execution::ComposedAgentFailure {
                    error,
                    model_requests,
                    tool_evidence,
                } = error;
                if matches!(error, scriberig_agent_runtime::AgentError::Cancelled) {
                    let _ = service.operations().cancel(&handle.operation);
                    let _ = service.fail_local_agent_with_evidence(
                        &handle,
                        error.to_string(),
                        model_requests,
                        tool_evidence,
                    );
                    eprintln!("agent operation cancelled: {}", handle.operation);
                    std::process::exit(130);
                }
                let _ = service.fail_local_agent_with_evidence(
                    &handle,
                    error.to_string(),
                    model_requests,
                    tool_evidence,
                );
                return Err(Box::new(error));
            }
        };
    service.complete_local_agent_with_evidence(
        &handle,
        runtime_result.output,
        Vec::new(),
        runtime_result.tool_evidence,
        runtime_result.model_requests,
    )?;
    let operation = handle.operation;
    let record = service.operations().get(&operation)?;
    let result = record
        .agent_result
        .ok_or("agent operation completed without a result")?;
    let format = args.format;
    println!(
        "{}",
        scriberig_cli::output::render_agent_result(&result, &record.reference.to_string(), format,)
    );
    eprintln!(
        "{}",
        scriberig_cli::output::render_agent_diagnostics(&result, &record.reference.to_string())
    );
    Ok(())
}

/// Executes one branch using an upstream provider and semantic broker.
///
/// - `args`: Provider and prompt configuration.
/// - `context`: Authorized application context for the runtime.
///
/// The completed provider-neutral runtime result.
///
/// Returns a provider, tool, budget, or cancellation failure.
fn run_composed_model<B: scriberig_app::ApplicationBackend + Sync>(
    args: &scriberig_cli::RunArgs,
    service: &scriberig_app::ApplicationService<B>,
    project_store: &Arc<SqliteStore>,
    context: &ApplicationContext,
    scope: &ChildExecutionScope,
    cancellation: &std::sync::atomic::AtomicBool,
) -> Result<scriberig_agent_runtime::AgentResult, main_agent_execution::ComposedAgentFailure> {
    let output_callback =
        (matches!(args.format, scriberig_cli::RunFormat::Human) && !args.no_stream).then(|| {
            std::sync::Arc::new(|text: &str| eprintln!("agent output: {text}"))
                as std::sync::Arc<dyn Fn(&str) + Send + Sync>
        });
    execute_model(
        args,
        service,
        project_store,
        context,
        scope,
        cancellation,
        output_callback,
    )
}

/// Allocates a fresh session identity for one local command invocation.
///
/// A session identity derived from the current process time.
///
/// Returns an error when the system clock is before the Unix epoch.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn fresh_session_id() -> Result<scriberig_core::EntityId, std::time::SystemTimeError> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&nanos.to_be_bytes());
    Ok(scriberig_core::EntityId::from_bytes(bytes))
}

/// Builds the least-authority context for local read-only product surfaces.
///
/// - `principal`: Stable identity assigned to the local surface.
///
/// An application context granting inspection only.
/// # Returns
/// The operation result.
fn local_read_context(principal: &str) -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions: [
                CapabilityAction::Inspect,
                CapabilityAction::Custom("agent_start".to_owned()),
                CapabilityAction::Custom("agent_continue".to_owned()),
                CapabilityAction::Custom("session_branch_tree".to_owned()),
            ]
            .into_iter()
            .collect::<BTreeSet<_>>(),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        principal: Some(scriberig_app::Principal::new(principal)),
        ..ApplicationContext::empty()
    }
}

/// Builds the least-authority action set for the integrated agent runtime.
///
/// Scoped actions for inspection, branch execution, and bounded observation.
/// # Returns
/// The operation result.
fn agent_actions() -> BTreeSet<CapabilityAction> {
    [
        CapabilityAction::Inspect,
        CapabilityAction::Query,
        CapabilityAction::Custom("agent.delegate".to_owned()),
        CapabilityAction::Custom("domain_operation".to_owned()),
        CapabilityAction::Custom("runtime_capabilities".to_owned()),
        CapabilityAction::Custom("operation_status".to_owned()),
        CapabilityAction::Custom("operation_cancel".to_owned()),
        CapabilityAction::Custom("event_subscription".to_owned()),
        CapabilityAction::Custom("agent_start".to_owned()),
        CapabilityAction::Custom("agent_continue".to_owned()),
        CapabilityAction::Custom("session_branch_tree".to_owned()),
        CapabilityAction::Custom("session_branch_activity".to_owned()),
        CapabilityAction::Custom("agent_event_subscription".to_owned()),
        CapabilityAction::Custom("agent_streaming_output".to_owned()),
        CapabilityAction::Custom("project_scan".to_owned()),
        CapabilityAction::Custom("project_manifest".to_owned()),
        CapabilityAction::Custom("project_import".to_owned()),
        CapabilityAction::Custom("project_workspace".to_owned()),
        CapabilityAction::Custom("project_rescan".to_owned()),
        CapabilityAction::Custom("project_repair_file".to_owned()),
        CapabilityAction::Custom("project_propose".to_owned()),
        CapabilityAction::Custom("project_merge_candidates".to_owned()),
        CapabilityAction::Custom("project_candidate".to_owned()),
        CapabilityAction::Custom("project_validate_candidate".to_owned()),
        CapabilityAction::Custom("project_resolve_dependencies".to_owned()),
        CapabilityAction::Custom("semantic_find".to_owned()),
        CapabilityAction::Custom("semantic_render".to_owned()),
    ]
    .into_iter()
    .collect()
}
