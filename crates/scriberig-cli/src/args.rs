//! Defines the commands and arguments accepted by the Scriberig CLI.

use std::collections::BTreeSet;

use clap::{Args, Parser, Subcommand};

use crate::project::ProjectArgs;
use crate::secrets::SecretsArgs;
use crate::trust;

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
/// - `project_roots`: Filesystem roots allowed for project onboarding.
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
    /// Filesystem root permitted for project scans and relinks; repeatable.
    #[arg(long = "project-root")]
    pub project_roots: Vec<std::path::PathBuf>,
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
    /// Optional local filesystem project to register and scan before startup.
    #[arg(long, requires = "db")]
    pub project: Option<std::path::PathBuf>,
    /// Import the selected project before starting the TUI.
    #[arg(long = "import", requires = "project")]
    pub import_project: bool,
    /// Restrict project onboarding to existing Git-indexed files.
    #[arg(long, requires = "project")]
    pub git_tracked: bool,
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
///
/// # Public members
/// - `prompt`: User prompt forwarded to the operation.
/// - `db`: Local database path.
/// - `format`: Output representation.
/// - `no_stream`: Whether transient output is disabled.
/// - `provider`: Safe provider profile identifier.
/// - `model`: Provider model identifier.
/// - `base_url`: Provider endpoint URL.
/// - `api_key_env`: Environment variable containing credentials.
/// - `credential_ref`: Explicit indirect credential selector.
/// - `credential`: Resolved typed credential reference.
/// - `explicit_cli`: CLI options explicitly supplied by the caller.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Args)]
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
    #[arg(long, conflicts_with = "credential_ref")]
    pub api_key_env: Option<String>,
    /// Indirect credential reference, or `none` to clear an inherited credential.
    #[arg(long, conflicts_with = "api_key_env")]
    pub credential_ref: Option<String>,
    /// Typed reference resolved without reading a secret.
    #[arg(skip)]
    pub credential: Option<scriberig_secrets::SecretRef>,
    /// Explicitly supplied CLI option names, captured for configuration precedence.
    #[arg(skip)]
    pub explicit_cli: BTreeSet<String>,
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
