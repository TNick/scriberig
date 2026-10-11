//! Defines command parsing and dispatch without owning application business logic.

use clap::{Args, Parser, Subcommand};

// Block: Define the stable top-level command grammar exposed by the native executable.
/// Parse the command line accepted by the `srig` executable.
///
/// # Public members
/// None.
///
/// # Private members
/// - `command`: Product mode selected by the caller.
#[derive(Debug, Parser)]
#[command(name = "srig", version, about = "Universal semantic knowledge and agent runtime", arg_required_else_help = true)]
struct Cli {
    /// Product mode selected by the caller.
    #[command(subcommand)]
    command: Command,
}

// Block: Enumerate product modes without implementing their runtime behavior yet.
/// Select a native Scriberig product mode.
#[derive(Debug, Subcommand)]
enum Command {
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
}

// Block: Capture API server endpoint options while server startup remains a stub.
/// Configure the API server startup stub.
///
/// # Public members
/// None.
///
/// # Private members
/// - `host`: Interface address that the eventual server should bind.
/// - `port`: TCP port that the eventual server should bind.
#[derive(Debug, Args)]
struct ServeArgs {
    /// Interface address that the eventual server should bind.
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    /// TCP port that the eventual server should bind.
    #[arg(long, default_value_t = 7337)]
    port: u16,
}

// Block: Capture a prompt consistently for one-shot and integrated-agent command stubs.
/// Store a prompt accepted by a prompt-driven command.
///
/// # Public members
/// None.
///
/// # Private members
/// - `prompt`: User prompt forwarded by a future runtime implementation.
#[derive(Debug, Args)]
struct PromptArgs {
    /// User prompt forwarded by a future runtime implementation.
    prompt: String,
}

// Block: Parse the process command line and dispatch to the currently selected stub.
/// Run the `srig` command-line entry point.
pub fn run() {
    let cli = Cli::parse();
    dispatch(cli.command);
}

// Block: Keep mode handling explicit while each product runtime is still unimplemented.
/// Dispatch a parsed product mode to its current stub behavior.
///
/// # Parameters
/// - `command`: Parsed product mode and its command-specific arguments.
fn dispatch(command: Command) {
    match command {
        Command::Tui => println!("srig tui: terminal UI mode is not implemented yet."),
        Command::Serve(args) => println!(
            "srig serve: API server mode is not implemented yet (requested {}:{}).",
            args.host, args.port
        ),
        Command::Run(args) => println!(
            "srig run: one-shot mode is not implemented yet (accepted {} prompt bytes).",
            args.prompt.len()
        ),
        Command::Agent(args) => println!(
            "srig agent: agent mode is not implemented yet (accepted {} prompt bytes).",
            args.prompt.len()
        ),
        Command::Maintenance => {
            println!("srig maintenance: maintenance mode is not implemented yet.");
        }
    }
}
