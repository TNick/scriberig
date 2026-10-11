//! Composes the integrated Scriberig executable from reusable and product runtime crates.

use std::collections::BTreeSet;

use scriberig_app::{
    ApplicationContext, ApplicationService, Capability, CapabilityAction, CapabilityConstraints,
    CapabilityContext, CapabilityScope, GuardedQueryService, StoreInspectionBackend,
};
use scriberig_store_sqlite::SqliteStore;

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
            scriberig_server::serve(&args.host, args.port, &args.db)?;
        }
        scriberig_cli::Command::Tui => {
            println!("srig tui: terminal UI mode is not implemented yet.");
        }
        scriberig_cli::Command::Run(args) => {
            println!(
                "srig run: one-shot mode is not implemented yet (accepted {} prompt bytes).",
                args.prompt.len()
            );
        }
        scriberig_cli::Command::Agent(args) => {
            println!(
                "srig agent: agent mode is not implemented yet (accepted {} prompt bytes).",
                args.prompt.len()
            );
        }
        scriberig_cli::Command::Maintenance => {
            println!("srig maintenance: maintenance mode is not implemented yet.");
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
