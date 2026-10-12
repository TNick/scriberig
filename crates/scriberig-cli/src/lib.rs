//! Defines command parsing and dispatch without owning application business logic.

use clap::{Args, Parser, Subcommand};

pub mod inspect;
pub mod output;
pub mod project;
pub mod trust;

pub use project::{
    ProjectArgs, ProjectCommand, ProjectDatabaseArgs, ProjectRelinkArgs, ProjectScanArgs,
    ProjectSelectorArgs,
};

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
    Tui(TuiArgs),
    /// Start the API server.
    Serve(ServeArgs),
    /// Run one prompt in headless one-shot mode.
    Run(RunArgs),
    /// Run one prompt through the integrated agent runtime.
    Agent(RunArgs),
    /// Run repository or database maintenance operations.
    Maintenance(MaintenanceArgs),
    /// Inspect one entity and its trust metadata.
    Inspect(InspectArgs),
    /// Discover capabilities exposed by a remote application.
    Capabilities(RemoteArgs),
    /// Observe or cancel one remote operation.
    Operation(OperationArgs),
    /// Read buffered lifecycle or agent streaming events.
    Events(EventsArgs),
    /// Read durable session and branch state.
    Session(SessionArgs),
    /// Continue an agent operation on an existing remote branch.
    AgentContinue(AgentContinueArgs),
    /// Manage projects stored in a shared database.
    Project(ProjectArgs),
}

/// Configures a command that uses the public HTTP protocol endpoint.
#[derive(Debug, Args)]
pub struct RemoteArgs {
    /// Public protocol endpoint used by the command.
    #[arg(long, default_value = "http://127.0.0.1:7337")]
    pub connect: String,
    /// Render the response as machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

/// Selects an operation lifecycle action.
#[derive(Debug, Subcommand)]
pub enum OperationCommand {
    /// Read one operation's current status.
    Status {
        /// Stable operation identifier.
        operation: String,
        /// Remote endpoint and output options.
        #[command(flatten)]
        remote: RemoteArgs,
    },
    /// Request cancellation of one operation.
    Cancel {
        /// Stable operation identifier.
        operation: String,
        /// Remote endpoint and output options.
        #[command(flatten)]
        remote: RemoteArgs,
    },
}

/// Selects an operation lifecycle command.
#[derive(Debug, Args)]
pub struct OperationArgs {
    /// Selected lifecycle action.
    #[command(subcommand)]
    pub command: OperationCommand,
}

/// Configures a buffered event observation request.
#[derive(Debug, Args)]
pub struct EventsArgs {
    /// Optional event-kind filter; repeat to select multiple kinds.
    #[arg(long = "kind")]
    pub event_kinds: Vec<String>,
    /// Return events newer than this cursor.
    #[arg(long)]
    pub after: Option<u64>,
    /// Remote endpoint and output options.
    #[command(flatten)]
    pub remote: RemoteArgs,
}

/// Selects a durable session read operation.
#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    /// Read the complete session branch tree and activity summaries.
    Tree {
        /// Stable session identifier.
        session: String,
        /// Remote endpoint and output options.
        #[command(flatten)]
        remote: RemoteArgs,
    },
    /// Read activity for one branch subtree.
    Activity {
        /// Stable session identifier.
        session: String,
        /// Stable branch identifier.
        branch: String,
        /// Remote endpoint and output options.
        #[command(flatten)]
        remote: RemoteArgs,
    },
}

/// Configures a durable session read command.
#[derive(Debug, Args)]
pub struct SessionArgs {
    /// Selected session read action.
    #[command(subcommand)]
    pub command: SessionCommand,
}

/// Configures continuation of an existing agent branch.
#[derive(Debug, Args)]
pub struct AgentContinueArgs {
    /// Stable session identifier.
    pub session: String,
    /// Stable branch identifier receiving the follow-up.
    pub branch: String,
    /// Follow-up instruction.
    pub prompt: String,
    /// Remote endpoint and output options.
    #[command(flatten)]
    pub remote: RemoteArgs,
}

/// Configure the API server startup.
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
    /// Exact browser origin allowed for cross-origin protocol requests.
    #[arg(long)]
    pub allow_origin: Option<String>,
}

/// Configure the local application-backed terminal UI.
#[derive(Debug, Args)]
pub struct TuiArgs {
    /// SQLite database path used by the local TUI composition.
    #[arg(
        long,
        default_value = "playground/dev-app-runtime/scriberig.sqlite",
        conflicts_with = "connect"
    )]
    pub db: std::path::PathBuf,
    /// Public protocol endpoint used by the remote TUI composition.
    #[arg(long, conflicts_with = "db")]
    pub connect: Option<String>,
}

/// Configure canonical-state maintenance.
///
/// # Public members
/// - `db`: SQLite database path to inspect.
///
/// # Private members
/// None.
#[derive(Debug, Args)]
pub struct MaintenanceArgs {
    /// SQLite database path to check.
    #[arg(long, default_value = "playground/dev-app-runtime/scriberig.sqlite")]
    pub db: std::path::PathBuf,
}

/// Store a prompt accepted by a prompt-driven command.
///
/// # Public members
/// - `prompt`: User prompt forwarded to the composed runtime.
///
/// # Private members
/// None.
#[derive(Debug, Args)]
pub struct PromptArgs {
    /// User prompt forwarded to the composed runtime.
    pub prompt: String,
}

/// Configures a headless shared agent operation.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// User prompt forwarded to the shared agent operation.
    pub prompt: String,
    /// SQLite database path used by the local application composition.
    #[arg(long, default_value = "playground/dev-app-runtime/scriberig.sqlite")]
    pub db: std::path::PathBuf,
    /// Output representation for the completed operation.
    #[arg(long, value_enum, default_value_t = RunFormat::Human)]
    pub format: RunFormat,
    /// Disable transient provider output while retaining the shared operation.
    #[arg(long)]
    pub no_stream: bool,
    /// Safe provider profile identifier used in diagnostics.
    #[arg(long, default_value = "default")]
    pub provider: String,
    /// Provider model identifier used when `--base-url` is supplied.
    #[arg(long, default_value = "default")]
    pub model: String,
    /// OpenAI-compatible provider base URL for real model execution.
    #[arg(long)]
    pub base_url: Option<String>,
    /// Environment variable containing the provider credential.
    #[arg(long, requires = "base_url")]
    pub api_key_env: Option<String>,
}

/// Selects the one-shot command output representation.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum RunFormat {
    /// Render final text and stable handles for a human.
    Human,
    /// Render one machine-readable JSON object.
    Json,
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

#[cfg(test)]
mod tests {
    use super::{Cli, Command, RunFormat};
    use clap::Parser;

    /// Verifies headless provider options remain part of the native command.
    #[test]
    fn run_parses_provider_configuration_without_secret_value() {
        let cli = Cli::try_parse_from([
            "srig",
            "run",
            "hello",
            "--provider",
            "local",
            "--model",
            "model-a",
            "--base-url",
            "http://127.0.0.1:9000/v1",
            "--api-key-env",
            "PROVIDER_KEY",
            "--format",
            "json",
            "--no-stream",
        ])
        .expect("run arguments");

        let Command::Run(args) = cli.command else {
            panic!("expected run command");
        };
        assert_eq!(args.prompt, "hello");
        assert_eq!(args.provider, "local");
        assert_eq!(args.model, "model-a");
        assert_eq!(args.api_key_env.as_deref(), Some("PROVIDER_KEY"));
        assert!(args.no_stream);
        assert!(matches!(args.format, RunFormat::Json));
    }
}
