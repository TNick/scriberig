//! Composes the integrated Scriberig executable from reusable and product runtime crates.

use std::collections::BTreeSet;
use std::sync::Arc;

use scriberig_app::{
    ApplicationContext, ApplicationService, Capability, CapabilityAction, CapabilityConstraints,
    CapabilityContext, CapabilityScope, GuardedQueryService, StoreInspectionBackend,
};
use scriberig_store_sqlite::SqliteStore;

mod agent_config;
mod agent_context;
mod agent_profile_composition;
mod agent_profile_recovery_command;
mod agent_tool;
mod agent_tool_helpers;
#[cfg(test)]
#[path = "artifact_agent_tests.rs"]
mod artifact_agent_tests;
mod bootstrap_failure_provider;
#[cfg(test)]
#[path = "cad_agent_tests.rs"]
mod cad_agent_tests;
mod child_runtime;
mod deterministic_provider;
mod doctor_command;
#[cfg(test)]
#[path = "document_agent_tests.rs"]
mod document_agent_tests;
#[cfg(test)]
#[path = "geo_agent_tests.rs"]
mod geo_agent_tests;
mod logging;
mod main_agent_execution;
mod native_scenario;
mod native_tui_agent;
mod projection_style_composition;
#[cfg(test)]
#[path = "qgis_agent_tests.rs"]
mod qgis_agent_tests;
mod remote_commands;
mod scenario_helpers;
mod secrets_command;
mod self_hosting_provider;
mod shared_agent_command;
#[cfg(test)]
#[path = "spreadsheet_agent_tests.rs"]
mod spreadsheet_agent_tests;
mod tui_command;
#[cfg(test)]
pub(crate) use shared_agent_command::run_composed_model;
use shared_agent_command::run_shared_agent;

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
    logging::initialize();
    #[cfg(feature = "isolated-plugin-worker")]
    let arguments = std::env::args().collect::<Vec<_>>();
    #[cfg(feature = "isolated-plugin-worker")]
    if scriberig_agent_runtime::plugin_worker::run_worker_mode(&arguments)
        .map_err(std::io::Error::other)?
    {
        return Ok(());
    }
    let command = scriberig_cli::parse();
    match command {
        scriberig_cli::Command::Serve(args) => {
            let store = Arc::new(SqliteStore::open(&args.db)?);
            let runner = Arc::new(native_tui_agent::NativeTuiAgentRunner::from_environment(
                Arc::clone(&store),
            )?);
            let provider_extension_runtime = runner.provider_extension_runtime();
            let readiness_runner = Arc::clone(&runner);
            let provider_readiness_handler: Arc<scriberig_server::ProviderReadinessHandler> =
                Arc::new(move || {
                    scriberig_tui::AgentCommandRunner::<
                        scriberig_server::LocalApplicationBackend,
                    >::provider_readiness(readiness_runner.as_ref())
                });
            let agent_execution_runner = Arc::clone(&runner);
            let agent_execution_handler: Arc<
                scriberig_server::AgentExecutionHandler<scriberig_server::LocalApplicationService>,
            > = Arc::new(move |service, context, request, accepted_response| {
                let mut runtime_context = context.clone();
                runtime_context.principal = Some(scriberig_app::Principal::new("local-http-agent"));
                runtime_context.capabilities.grants.push(Capability {
                    scope: CapabilityScope::Project,
                    actions: agent_actions(),
                    constraints: CapabilityConstraints::unrestricted(),
                });
                let accepted = accepted_response.map_or_else(
                    || service.begin_agent_execution(&runtime_context, request),
                    |accepted| main_agent_execution::from_protocol_acceptance(request, accepted),
                )?;
                let _execution = scriberig_tui::AgentCommandRunner::<
                    scriberig_server::LocalApplicationBackend,
                >::execute_accepted(
                    agent_execution_runner.as_ref(),
                    service,
                    &runtime_context,
                    &accepted,
                );
                scriberig_server::ApplicationService::operation_status(
                    service,
                    &accepted.operation.to_string(),
                )
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
            scriberig_server::serve_with_agent_execution(
                &args.host,
                args.port,
                &args.db,
                args.allow_origin.as_deref(),
                &args.project_roots,
                Some(agent_execution_handler),
                Some(plan_lifecycle_handler),
                scriberig_server::ServerProviderCatalog {
                    provider_extension_runtime,
                    provider_readiness_handler: Some(provider_readiness_handler),
                    cad_provider_registry: scriberig_app::CadProviderRegistry::new(),
                },
                projection_style_composition::domain_projection_style_configuration(&args.db)?,
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
        scriberig_cli::Command::RecoverProfile(args) => {
            agent_profile_recovery_command::run(&args)?;
        }
    }
    Ok(())
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
        CapabilityAction::Custom("cad.update_line_geometry".to_owned()),
        CapabilityAction::Custom("cad.change_entity_layer".to_owned()),
        CapabilityAction::Custom("spreadsheet.set_cell_value".to_owned()),
        CapabilityAction::Custom("document.replace_paragraph".to_owned()),
        CapabilityAction::Custom("document.update_table_cell".to_owned()),
        CapabilityAction::Custom("spreadsheet.read_range".to_owned()),
        CapabilityAction::Custom("spreadsheet.rename_sheet".to_owned()),
        CapabilityAction::Custom("qgis.rename_layer_reference".to_owned()),
        CapabilityAction::Custom("artifact.inspect".to_owned()),
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
