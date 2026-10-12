//! Verifies the complete stable native command grammar.

use clap::{CommandFactory, Parser};
use scriberig_cli::Cli;

/// Verifies every stable top-level command parses through the public CLI.
#[test]
fn stable_commands_parse() {
    let cases = [
        ("tui", vec!["srig", "tui"]),
        ("serve", vec!["srig", "serve"]),
        ("run", vec!["srig", "run", "prompt"]),
        ("agent", vec!["srig", "agent", "prompt"]),
        ("maintenance", vec!["srig", "maintenance"]),
        (
            "inspect",
            vec!["srig", "inspect", "ent_01010101010101010101010101010101"],
        ),
        ("capabilities", vec!["srig", "capabilities"]),
        ("operation", vec!["srig", "operation", "status", "op-1"]),
        (
            "events",
            vec!["srig", "events", "--kind", "agent_output_delta"],
        ),
        (
            "session",
            vec![
                "srig",
                "session",
                "tree",
                "ent_01010101010101010101010101010101",
            ],
        ),
        (
            "agent-continue",
            vec![
                "srig",
                "agent-continue",
                "ent_01010101010101010101010101010101",
                "ent_01010101010101010101010101010101",
                "prompt",
            ],
        ),
    ];

    for (name, arguments) in cases {
        let parsed = Cli::try_parse_from(arguments).expect(name);
        let debug_name = name.replace('-', "");
        assert!(format!("{parsed:?}").to_lowercase().contains(&debug_name));
    }
}

/// Verifies standard help and version flags are exposed by Clap.
#[test]
fn standard_help_and_version_are_available() {
    let help = Cli::try_parse_from(["srig", "--help"]).expect_err("help exits early");
    assert!(help.to_string().contains("Usage: srig"));

    let version = Cli::try_parse_from(["srig", "--version"]).expect_err("version exits early");
    assert!(version.to_string().contains(env!("CARGO_PKG_VERSION")));

    let mut command = Cli::command();
    let help = command.render_help().to_string();
    for name in [
        "tui",
        "serve",
        "run",
        "agent",
        "maintenance",
        "capabilities",
        "operation",
        "events",
        "session",
        "agent-continue",
    ] {
        assert!(help.contains(name), "help omits {name}");
    }
}

/// Verifies malformed command arguments are rejected before dispatch.
#[test]
fn malformed_arguments_are_rejected() {
    assert!(Cli::try_parse_from(["srig", "run"]).is_err());
    assert!(Cli::try_parse_from(["srig", "serve", "--port", "not-a-port"]).is_err());
    assert!(
        Cli::try_parse_from([
            "srig",
            "tui",
            "--db",
            "one.sqlite",
            "--connect",
            "http://127.0.0.1:7337",
        ])
        .is_err()
    );
}
