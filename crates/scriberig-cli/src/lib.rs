//! Defines command parsing and dispatch without owning application business logic.

use clap::{Args, Parser, Subcommand};

pub mod inspect;
pub mod output;
pub mod trust;

/// Reports a CLI dispatch failure while preserving its process status.
#[derive(Debug)]
pub struct DispatchError {
    /// Human-readable safe failure text.
    pub message: String,
    /// Process status associated with the failure.
    pub exit_code: i32,
}

/// Dispatches an inspect command through an injected application service.
///
/// # Parameters
/// - `service`: Application query service supplied by composition.
/// - `context`: Caller and capability context.
/// - `command`: Parsed product command.
///
/// # Returns
/// Optional output for commands handled by this dispatcher.
///
/// # Errors
/// Returns a structured dispatch failure for unsupported or denied commands.
pub fn dispatch<S: scriberig_app::QueryService>(
    service: &S,
    context: &scriberig_app::ApplicationContext,
    command: Command,
) -> Result<Option<String>, DispatchError> {
    match command {
        Command::Inspect(args) => {
            inspect::run(service, context, &args)
                .map(Some)
                .map_err(|error| DispatchError {
                    message: match &error {
                        trust::TrustCommandError::AuthorizationDenied(denial) => {
                            trust::render_denial(denial)
                        }
                        trust::TrustCommandError::Application(application) => {
                            application.to_string()
                        }
                    },
                    exit_code: error.exit_code(),
                })
        }
        _ => Err(DispatchError {
            message: "command is not available in this composition".to_owned(),
            exit_code: 1,
        }),
    }
}

/// Parse the command line accepted by the `srig` executable.
///
/// # Public members
/// None.
///
/// # Private members
/// - `command`: Product mode selected by the caller.
#[derive(Debug, Parser)]
#[command(name = "srig", version, about, arg_required_else_help = true)]
pub struct Cli {
    /// Product mode selected by the caller.
    #[command(subcommand)]
    command: Command,
}

/// Select a native Scriberig product mode.
///
/// # Public members
/// - `Tui`: Starts the terminal UI mode.
/// - `Serve`: Starts the API server.
/// - `Run`: Accepts one prompt in one-shot mode.
/// - `Agent`: Accepts one prompt in agent mode.
/// - `Maintenance`: Selects repository or database maintenance.
///
/// # Private members
/// None.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start the interactive terminal UI.
    Tui,
    /// Start the API server.
    Serve(ServeArgs),
    /// Run one prompt in headless one-shot mode.
    Run(PromptArgs),
    /// Run one prompt through the integrated agent runtime.
    Agent(PromptArgs),
    /// Run repository or database maintenance operations.
    Maintenance,
    /// Inspect one entity and its trust metadata.
    Inspect(InspectArgs),
}

/// Configure the API server startup stub.
///
/// # Public members
/// - `host`: Interface address that the server should bind.
/// - `port`: TCP port that the server should bind.
/// - `db`: SQLite database file for the server.
///
/// # Private members
/// None.
#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Interface address that the server should bind.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// TCP port that the server should bind.
    #[arg(long, default_value_t = 7337)]
    pub port: u16,
    /// SQLite database path used by the local API server.
    #[arg(long, default_value = "playground/dev-app-runtime/scriberig.sqlite")]
    pub db: std::path::PathBuf,
}

/// Store a prompt accepted by a prompt-driven command.
///
/// # Public members
/// - `prompt`: User prompt forwarded by a future runtime implementation.
///
/// # Private members
/// None.
#[derive(Debug, Args)]
pub struct PromptArgs {
    /// User prompt forwarded by a future runtime implementation.
    pub prompt: String,
}

/// Configure an entity inspection request.
#[derive(Debug, Args)]
pub struct InspectArgs {
    /// Entity identifier to inspect.
    pub entity: scriberig_core::EntityId,
    /// Exact snapshot to inspect when supplied.
    #[arg(long)]
    pub snapshot: Option<scriberig_core::SnapshotId>,
    /// Print provenance handles and deeper trust metadata.
    #[arg(long, value_enum, default_value_t = trust::InspectDepth::Compact)]
    pub depth: trust::InspectDepth,
    /// Output representation for the trust result.
    #[arg(long, value_enum, default_value_t = trust::OutputFormat::Human)]
    pub format: trust::OutputFormat,
    /// SQLite database path used by the local inspection composition.
    #[arg(long, default_value = "playground/dev-app-runtime/scriberig.sqlite")]
    pub db: std::path::PathBuf,
}

/// Parse command-line input for the `srig` executable.
///
/// # Returns
/// The selected product mode and its arguments.
#[must_use]
pub fn parse() -> Command {
    Cli::parse().command
}
