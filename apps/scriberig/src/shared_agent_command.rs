//! Runs the shared application agent boundary for the headless product command.

use std::sync::Arc;

use scriberig_agent_runtime::ChildExecutionScope;
use scriberig_app::{
    ApplicationBackendComposition, ApplicationContext, ApplicationService, Capability,
    CapabilityConstraints, CapabilityId, CapabilityScope, StoreInspectionBackend,
    StoreOperationProvider,
};
use scriberig_store_sqlite::SqliteStore;

use super::{
    agent_actions, agent_config, agent_profile_composition, fresh_session_id, main_agent_execution,
    native_tui_agent, projection_style_composition,
};
use main_agent_execution::execute_model;

#[expect(
    clippy::too_many_lines,
    reason = "headless composition preserves the shared application execution sequence"
)]
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
pub(super) fn run_shared_agent(
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
    let service = ApplicationService::new(backend)
        .with_session_store(session_store)
        .with_agent_profile_provider(agent_profile_composition::provider(Arc::clone(&store)))
        .with_projection_style_configuration(
            projection_style_composition::domain_projection_style_configuration(&args.db)?,
        );
    let session = fresh_session_id()?;
    let mut context = ApplicationContext {
        capabilities: scriberig_app::CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions: agent_actions(),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        principal: Some(scriberig_app::Principal::new("local-agent")),
        ..ApplicationContext::empty()
    };
    context
        .capabilities
        .granted
        .push(CapabilityId::new("agent_start").expect("agent-start capability"));
    native_tui_agent::grant_configured_plugin_capability(&mut context);
    let request = scriberig_app::AgentCommandRequest {
        session,
        branch: None,
        anchor: None,
        prompt: args.prompt.clone(),
        selection: None,
    };
    let handle = service.begin_agent_execution(&context, &request)?;
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
        provider_resolver: None,
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
pub(crate) fn run_composed_model<B: scriberig_app::ApplicationBackend + Send + Sync + 'static>(
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
