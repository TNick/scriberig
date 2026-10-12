//! Verifies the public TUI command grammar.

use clap::Parser;
use scriberig_cli::Cli;

/// Verifies the TUI command accepts its local database option.
#[test]
fn tui_parses_database_path() {
    let cli = Cli::try_parse_from(["srig", "tui", "--db", "test.sqlite"]).expect("tui arguments");
    assert!(format!("{cli:?}").contains("test.sqlite"));
}

/// Verifies the remote endpoint is accepted as an alternative selector.
#[test]
fn tui_parses_remote_endpoint() {
    let cli = Cli::try_parse_from(["srig", "tui", "--connect", "http://127.0.0.1:7337"])
        .expect("remote TUI arguments");
    assert!(format!("{cli:?}").contains("http://127.0.0.1:7337"));
}

/// Verifies local and remote selectors cannot be combined.
#[test]
fn tui_rejects_database_and_remote_endpoint_together() {
    let result = Cli::try_parse_from([
        "srig",
        "tui",
        "--db",
        "test.sqlite",
        "--connect",
        "http://127.0.0.1:7337",
    ]);
    assert!(result.is_err());
}

/// Verifies maintenance accepts an explicit SQLite database path.
#[test]
fn maintenance_parses_database_path() {
    let cli = Cli::try_parse_from(["srig", "maintenance", "--db", "custom.sqlite"])
        .expect("maintenance arguments");
    assert!(format!("{cli:?}").contains("custom.sqlite"));
}
