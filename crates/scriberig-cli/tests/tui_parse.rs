//! Verifies the public TUI command grammar.

use clap::Parser;
use scriberig_cli::Cli;

/// Verifies the TUI command accepts its local database option.
#[test]
fn tui_parses_database_path() {
    let cli = Cli::try_parse_from(["srig", "tui", "--db", "test.sqlite"]).expect("tui arguments");
    assert!(format!("{cli:?}").contains("test.sqlite"));
}
