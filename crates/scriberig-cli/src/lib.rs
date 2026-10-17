//! Defines command parsing and dispatch without owning application business logic.

use clap::{CommandFactory, FromArgMatches};
use std::collections::BTreeSet;

pub mod args;
pub mod config;

pub use args::*;
pub mod inspect;
pub mod output;
pub mod project;
pub mod secrets;
pub mod trust;

pub use project::{
    ProjectArgs, ProjectCommand, ProjectDatabaseArgs, ProjectImportArgs, ProjectRelinkArgs,
    ProjectScanArgs, ProjectSelectorArgs,
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

/// Parse command-line input for the `srig` executable.
///
/// # Returns
/// The selected product mode and its arguments.
///
/// # Panics
/// Panics only if the generated Clap parser cannot deserialize its own matches.
#[must_use]
pub fn parse() -> Command {
    let parser = Cli::command();
    let matches = parser.get_matches();
    let mut command = Cli::from_arg_matches(&matches)
        .expect("Clap matches must deserialize into the CLI model")
        .command;
    let explicit = matches
        .subcommand()
        .map_or_else(BTreeSet::new, |(name, command)| {
            if matches!(name, "run" | "agent") {
                explicit_run_options(command)
            } else {
                BTreeSet::new()
            }
        });
    match &mut command {
        Command::Run(args) | Command::Agent(args) => args.explicit_cli = explicit,
        _ => {}
    }
    command
}

/// Collects options supplied directly to a run-like command.
///
/// # Parameters
/// - `matches`: Parsed Clap matches for the selected command.
///
/// # Returns
/// Explicit option identifiers used for configuration precedence.
fn explicit_run_options(matches: &clap::ArgMatches) -> BTreeSet<String> {
    [
        "db",
        "format",
        "no_stream",
        "provider",
        "model",
        "base_url",
        "api_key_env",
        "credential_ref",
    ]
    .into_iter()
    .filter(|name| matches.value_source(name) == Some(clap::parser::ValueSource::CommandLine))
    .map(str::to_owned)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command, ConfigCommand, RunFormat, explicit_run_options};
    use clap::{CommandFactory, Parser};

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

    /// Verifies the effective configuration inspection command parses.
    #[test]
    fn config_show_parses_json_output() {
        let cli = Cli::try_parse_from(["srig", "config", "show", "--format", "json"])
            .expect("config show arguments");
        let Command::Config(args) = cli.command else {
            panic!("expected config command");
        };
        let ConfigCommand::Show(show) = args.command;
        assert!(matches!(show.format, RunFormat::Json));
    }

    /// Verifies parser defaults do not masquerade as explicit CLI overrides.
    #[test]
    fn tracks_only_run_options_supplied_on_the_command_line() {
        let parser = Cli::command();
        let matches = parser
            .try_get_matches_from(["srig", "run", "hello", "--model", "chosen"])
            .expect("run matches");
        let run_matches = matches.subcommand_matches("run").expect("run subcommand");

        let explicit = explicit_run_options(run_matches);
        assert_eq!(explicit, ["model".to_owned()].into_iter().collect());
    }
}
