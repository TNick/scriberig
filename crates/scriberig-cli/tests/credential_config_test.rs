//! Verifies atomic reference merging, compatibility aliases, and safe parsing.

use clap::Parser;
use scriberig_cli::{
    Cli,
    config::{ConfigResolver, ConfigSource, render},
};
use std::collections::BTreeMap;

/// A higher-priority environment reference replaces the entire lower reference.
#[test]
fn references_replace_atomically_and_aliases_share_precedence() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let root = temporary.path().canonicalize().expect("canonical root");
    std::fs::write(
        root.join("config.yaml"),
        "credential:\n  source: keyring\n  name: lower\n",
    )
    .expect("config");
    let environment = BTreeMap::from([("SRIG_API_KEY_ENV".to_owned(), "UPPER_KEY".to_owned())]);
    let configuration = ConfigResolver::with_inputs(None, None, root, environment)
        .resolve(&BTreeMap::new())
        .expect("merged references");
    assert_eq!(
        configuration.entries["credential"].value,
        serde_json::json!({"source":"environment", "name":"UPPER_KEY"})
    );
    assert_eq!(
        configuration.entries["credential"].source,
        ConfigSource::Environment("SRIG_API_KEY_ENV".to_owned())
    );
    assert!(!configuration.entries.contains_key("api_key_env"));
}

/// Null explicitly masks an inherited credential and keeps its CLI provenance.
#[test]
fn null_clears_inherited_reference() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let root = temporary.path().canonicalize().expect("canonical root");
    let environment = BTreeMap::from([("SRIG_API_KEY_ENV".to_owned(), "LOWER_KEY".to_owned())]);
    let cli = BTreeMap::from([("credential".to_owned(), serde_json::Value::Null)]);
    let configuration = ConfigResolver::with_inputs(None, None, root, environment)
        .resolve(&cli)
        .expect("explicit null");
    assert!(configuration.entries["credential"].value.is_null());
    assert_eq!(
        configuration.entries["credential"].source,
        ConfigSource::CommandLine("--credential-ref".to_owned())
    );
}

/// Conflicting aliases in one source fail without exposing accidental secret input.
#[test]
fn conflicting_and_inline_references_fail_without_echoing_values() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let root = temporary.path().canonicalize().expect("canonical root");
    std::fs::write(
        root.join("config.json"),
        r#"{"api_key_env":"ENV_KEY","credential":{"source":"keyring","name":"work"}}"#,
    )
    .expect("config");
    let resolver = ConfigResolver::with_inputs(None, None, root.clone(), BTreeMap::new());
    assert!(
        resolver
            .resolve(&BTreeMap::new())
            .expect_err("conflict")
            .message
            .contains("same configuration layer")
    );
    std::fs::write(
        root.join("config.json"),
        r#"{"credential":{"source":"keyring","name":"work","value":"CANARY_DO_NOT_ECHO"}}"#,
    )
    .expect("inline input");
    let failure = resolver
        .resolve(&BTreeMap::new())
        .expect_err("inline secret rejected");
    assert!(!failure.message.contains("CANARY_DO_NOT_ECHO"));
}

/// Invalid partial references cannot borrow a source tag from a lower layer.
#[test]
fn partial_reference_cannot_inherit_fields() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let root = temporary.path().canonicalize().expect("canonical root");
    std::fs::write(
        root.join("config.json"),
        r#"{"credential":{"source":"keyring","name":"work"}}"#,
    )
    .expect("config");
    let cli = BTreeMap::from([("credential".to_owned(), serde_json::json!({"name":"other"}))]);
    let resolver = ConfigResolver::with_inputs(None, None, root, BTreeMap::new());
    assert!(resolver.resolve(&cli).is_err());
}

/// Inspection uses only references and never needs a live credential store.
#[test]
fn inspection_renders_unavailable_reference_without_resolving_it() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let root = temporary.path().canonicalize().expect("canonical root");
    let environment = BTreeMap::from([(
        "SRIG_CREDENTIAL".to_owned(),
        r#"{"source":"keyring","name":"missing-forever"}"#.to_owned(),
    )]);
    let configuration = ConfigResolver::with_inputs(None, None, root, environment)
        .resolve(&BTreeMap::new())
        .expect("reference only");
    assert!(
        render(&configuration, true)
            .expect("render")
            .contains("missing-forever")
    );
}

/// Lifecycle commands cannot accept raw secret values on the command line.
#[test]
fn lifecycle_parser_rejects_secret_value_arguments() {
    assert!(Cli::try_parse_from(["srig", "secrets", "set", "work", "--value", "CANARY"]).is_err());
    assert!(Cli::try_parse_from(["srig", "secrets", "set", "work", "--stdin"]).is_ok());
    assert!(
        Cli::try_parse_from([
            "srig",
            "run",
            "hello",
            "--credential-ref",
            "keyring:work",
            "--api-key-env",
            "ENV_KEY"
        ])
        .is_err()
    );
}
