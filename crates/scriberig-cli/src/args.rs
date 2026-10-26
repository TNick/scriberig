//! Defines the commands and arguments accepted by the Scriberig CLI.

use clap::{Args, Parser, Subcommand};

use crate::project::ProjectArgs;
use crate::secrets::SecretsArgs;
use crate::trust;

#[path = "args/operation.rs"]
mod operation;
pub use operation::*;

/// Parse the command line accepted by the `srig` executable.
///
/// # Public members
/// - `command`: Product mode selected by the caller.
///
/// # Private members
/// None.
#[derive(Debug, Parser)]
#[command(name = "srig", version, about, arg_required_else_help = true)]
pub struct Cli {
    /// Product mode selected by the caller.
    #[command(subcommand)]
    pub command: Command,
}

/// Select a native Scriberig product mode.
///
/// # Public members
/// - `Tui`: Starts the terminal UI mode.
/// - `Serve`: Starts the API server.
/// - `Run`: Accepts one prompt in one-shot mode.
/// - `Agent`: Accepts one prompt in agent mode.
/// - `Maintenance`: Selects repository or database maintenance.
/// - `Doctor`: Tests the configured model API and credential.
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
    /// Inspect resolved Scriberig configuration.
    Config(ConfigArgs),
    /// Manage local OS credentials and destination bindings.
    Secrets(SecretsArgs),
    /// Test the configured model API and credential with a minimal request.
    Doctor(DoctorArgs),
    /// Recover a legacy session profile using an explicit semantic template.
    RecoverProfile(RecoverProfileArgs),
}

/// Configures explicit recovery of one preserved legacy profile selection.
///
/// # Public members
/// - `session`: Exact session identifier containing the legacy record.
/// - `inspect`: Whether to display the preserved record and current revision.
/// - `revision`: Execution-selection revision observed during recovery review.
/// - `profile`: Operator-authored complete semantic profile JSON file.
/// - `provider`: Current upstream provider identifier.
/// - `model`: Current upstream model identifier.
/// - `thinking_level`: Optional current upstream thinking-level identifier.
/// - `db`: SQLite database used by the local composition.
/// - `yes`: Whether to skip interactive confirmation.
///
/// # Private members
/// None.
#[derive(Debug, Args)]
pub struct RecoverProfileArgs {
    /// Exact session identifier containing the legacy record.
    pub session: String,
    /// Display the exact recoverable profile identity and current selection revision.
    #[arg(long, conflicts_with_all = ["revision", "profile", "provider", "model", "thinking_level", "yes"])]
    pub inspect: bool,
    /// Execution-selection revision observed during recovery review.
    #[arg(long, required_unless_present = "inspect")]
    pub revision: Option<u64>,
    /// JSON file containing the complete semantic profile descriptor.
    #[arg(long, required_unless_present = "inspect")]
    pub profile: Option<std::path::PathBuf>,
    /// Current upstream provider identifier selected by the operator.
    #[arg(long, required_unless_present = "inspect")]
    pub provider: Option<String>,
    /// Current upstream model identifier selected by the operator.
    #[arg(long, required_unless_present = "inspect")]
    pub model: Option<String>,
    /// Optional current upstream thinking-level identifier.
    #[arg(long)]
    pub thinking_level: Option<String>,
    /// SQLite database path used by the local composition.
    #[arg(long, default_value = "playground/dev-app-runtime/scriberig.sqlite")]
    pub db: std::path::PathBuf,
    /// Skip the interactive confirmation prompt.
    #[arg(long)]
    pub yes: bool,
}

/// Configures a live provider connection diagnostic.
///
/// # Public members
/// - `format`: Human-readable or JSON report format.
///
/// # Private members
/// None.
#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Select human-readable or machine-readable output.
    #[arg(long, value_enum, default_value_t = RunFormat::Human)]
    pub format: RunFormat,
}

/// Configure resolved-configuration inspection.
///
/// # Public members
/// - `command`: Selected configuration operation.
///
/// # Private members
/// None.
#[derive(Debug, Args)]
pub struct ConfigArgs {
    /// Selected configuration operation.
    #[command(subcommand)]
    pub command: ConfigCommand,
}

/// Selects a configuration operation.
///
/// # Public members
/// - `Show`: Prints effective values and their source.
///
/// # Private members
/// None.
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the effective settings and the source of each property.
    Show(ConfigShowArgs),
}

/// Configures configuration inspection output.
///
/// # Public members
/// - `format`: Human-readable or JSON representation.
///
/// # Private members
/// None.
#[derive(Debug, Args)]
pub struct ConfigShowArgs {
    /// Select human-readable or JSON output.
    #[arg(long, value_enum, default_value_t = RunFormat::Human)]
    pub format: RunFormat,
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
