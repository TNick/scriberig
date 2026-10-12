//! Composes the integrated Scriberig executable from reusable and product runtime crates.

use std::collections::BTreeSet;
use std::sync::Arc;

use scriberig_agent_core::InspectTool;
use scriberig_agent_runtime::ChildExecutionScope;
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{CredentialSource, ProviderConfig};
use scriberig_app::{
    ApplicationBackendComposition, ApplicationContext, ApplicationService, Capability,
    CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityScope,
    GuardedQueryService, StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_store_sqlite::SqliteStore;

mod child_runtime;
use child_runtime::{LocalChildExecutor, inspect_catalog};

/// Start the integrated `srig` executable and report runtime failures.
///
/// # Returns
/// Unit after the selected command completes.
///
/// # Errors
/// Returns an error when the API server cannot start or serve a request.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let command = scriberig_cli::parse();
    match command {
        scriberig_cli::Command::Serve(args) => {
            scriberig_server::serve_with_origin(
                &args.host,
                args.port,
                &args.db,
                args.allow_origin.as_deref(),
            )?;
        }
        scriberig_cli::Command::Tui(args) => {
            if let Some(endpoint) = args.connect {
                let transport = scriberig_client::HttpTransport::from_endpoint(&endpoint)?;
                let client = scriberig_client::Client::new(transport);
                let service = scriberig_tui::RemoteAdapter::new(client);
                let context = local_read_context("remote-tui");
                scriberig_tui::run(
                    &service,
                    &context,
                    &scriberig_tui::TuiConfig {
                        interactive: true,
                        ..scriberig_tui::TuiConfig::default()
                    },
                )?;
                return Ok(());
            }
            let store = Arc::new(SqliteStore::open(&args.db)?);
            let fixture =
                scriberig_server::ensure_fixture(&store).map_err(std::io::Error::other)?;
            let backend = ApplicationBackendComposition::new(
                StoreInspectionBackend::new(Arc::clone(&store)),
                StoreOperationProvider::new(Arc::clone(&store)),
            );
            let service = ApplicationService::new(backend);
            let service = scriberig_tui::ApplicationOperationAdapter::new(service);
            let context = local_read_context("local-tui");
            scriberig_tui::run(
                &service,
                &context,
                &scriberig_tui::TuiConfig {
                    interactive: true,
                    initial_entity: Some(fixture.root_entity),
                    snapshot: Some(fixture.snapshot),
                },
            )?;
        }
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
    }
    Ok(())
}

/// Executes a headless prompt through the shared application agent boundary.
///
/// # Parameters
/// - `args`: Parsed prompt, database, and output settings.
///
/// # Returns
/// Unit after the terminal operation result is rendered.
///
/// # Errors
/// Returns an error when the local store, session operation, or output fails.
fn run_shared_agent(
    args: &scriberig_cli::RunArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
            actions: [
                CapabilityAction::Inspect,
                CapabilityAction::Custom("agent.delegate".to_owned()),
                CapabilityAction::Custom("agent_start".to_owned()),
                CapabilityAction::Custom("agent_continue".to_owned()),
                CapabilityAction::Custom("session_branch_tree".to_owned()),
            ]
            .into_iter()
            .collect::<BTreeSet<_>>(),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        principal: Some(scriberig_app::Principal::new("local-agent")),
        ..ApplicationContext::empty()
    };
    let cancellation = scriberig_agent_runtime::cancellation_flag();
    let cancellation_handler = Arc::clone(&cancellation);
    ctrlc::set_handler(move || {
        cancellation_handler.store(true, std::sync::atomic::Ordering::Release);
    })?;
    let request = scriberig_app::AgentCommandRequest {
        session,
        branch: None,
        prompt: args.prompt.clone(),
    };
    let handle = service.start_local_agent(&request)?;
    let scope = ChildExecutionScope {
        session,
        parent_branch: handle.root_branch,
        depth: 0,
        active_branches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let runtime_result = match run_composed_model(args, &service, &context, &scope, &cancellation) {
        Ok(result) => result,
        Err(error) => {
            if matches!(error, scriberig_agent_runtime::AgentError::Cancelled) {
                let _ = service.operations().cancel(&handle.operation);
                eprintln!("agent operation cancelled: {}", handle.operation);
                std::process::exit(130);
            }
            let _ = service.operations().fail(
                &handle.operation,
                scriberig_app::ApplicationError::Internal {
                    message: error.to_string(),
                },
            );
            return Err(Box::new(error));
        }
    };
    service.complete_local_agent(&handle, runtime_result.output)?;
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

/// Executes one local model loop using either the deterministic provider or a
/// configured OpenAI-compatible provider.
///
/// # Parameters
/// - `args`: Provider and prompt configuration.
/// - `context`: Authorized application context for the runtime.
///
/// # Returns
/// The completed provider-neutral runtime result.
///
/// # Errors
/// Returns a provider, tool, budget, or cancellation failure.
fn run_composed_model<B: scriberig_app::ApplicationBackend>(
    args: &scriberig_cli::RunArgs,
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    scope: &ChildExecutionScope,
    cancellation: &std::sync::atomic::AtomicBool,
) -> Result<scriberig_agent_runtime::AgentResult, scriberig_agent_runtime::AgentError> {
    if let Some(base_url) = &args.base_url {
        let credential_source = args
            .api_key_env
            .clone()
            .map_or(CredentialSource::None, CredentialSource::Environment);
        let config = ProviderConfig::new(
            args.provider.clone(),
            base_url,
            args.model.clone(),
            credential_source,
        );
        let provider =
            scriberig_agent_runtime::openai_compatible::OpenAiCompatibleProvider::new(config)
                .map_err(|error| {
                    scriberig_agent_runtime::AgentError::Provider(error.to_string())
                })?;
        return execute_model(
            provider,
            args.model.clone(),
            service,
            context,
            scope,
            &args.prompt,
            cancellation,
        );
    }
    scriberig_agent_runtime::AgentRuntime::new(
        scriberig_agent_runtime::EchoProvider,
        InspectTool::new(service),
        8,
    )
    .run(context, &args.prompt, cancellation)
}

/// Runs one provider-neutral model through the shared runtime loop.
///
/// # Parameters
/// - `provider`: Model decision provider.
/// - `context`: Trusted application context.
/// - `prompt`: Human instruction.
/// - `cancellation`: Shared cancellation flag.
///
/// # Returns
/// Completed runtime output and tool observations.
///
/// # Errors
/// Returns the runtime's structured failure.
fn execute_model<
    P: scriberig_agent_runtime::provider::ProviderAdapter + Clone,
    B: scriberig_app::ApplicationBackend,
>(
    provider: P,
    model: String,
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    scope: &ChildExecutionScope,
    prompt: &str,
    cancellation: &std::sync::atomic::AtomicBool,
) -> Result<scriberig_agent_runtime::AgentResult, scriberig_agent_runtime::AgentError> {
    let catalog = inspect_catalog();
    let cache = scriberig_agent_runtime::cache::ModelCache::new();
    let mut children = LocalChildExecutor {
        service,
        provider: provider.clone(),
        model: model.clone(),
        cache: cache.clone(),
    };
    scriberig_agent_runtime::AgentRuntime::new(
        ProviderModel::new(provider, model)
            .with_catalog(catalog)
            .with_capabilities(context.capabilities.clone())
            .with_cache(
                cache,
                scriberig_agent_runtime::cache::CachePolicy::ReuseExact,
            )
            .with_cache_namespace(scope.parent_branch.to_string()),
        InspectTool::new(service),
        8,
    )
    .run_with_child_executor_at(scope, context, prompt, cancellation, &mut children)
}

/// Allocates a fresh session identity for one local command invocation.
///
/// # Returns
/// A session identity derived from the current process time.
///
/// # Errors
/// Returns an error when the system clock is before the Unix epoch.
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
/// # Parameters
/// - `principal`: Stable identity assigned to the local surface.
///
/// # Returns
/// An application context granting inspection only.
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
