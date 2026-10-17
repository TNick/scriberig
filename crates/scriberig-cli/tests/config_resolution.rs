//! Verifies layered configuration precedence and parsing behavior.

use std::collections::BTreeMap;
use std::path::PathBuf;

use scriberig_cli::config::{ConfigResolver, ConfigSource, render};

/// Creates an isolated directory for one configuration test.
///
/// # Returns
/// A unique path under the platform temporary directory.
fn temporary_dir() -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("scriberig-config-{nonce}"));
    std::fs::create_dir_all(&path).expect("create temporary directory");
    path
}

/// Writes a config file with the requested name and contents.
///
/// # Parameters
/// - `directory`: Parent directory for the file.
/// - `name`: Supported configuration filename.
/// - `contents`: File contents to write.
///
/// # Returns
/// The path to the created file.
fn write_config(directory: &std::path::Path, name: &str, contents: &str) -> PathBuf {
    std::fs::create_dir_all(directory).expect("create config directory");
    let path = directory.join(name);
    std::fs::write(&path, contents).expect("write config file");
    path
}

/// Verifies each property follows the complete source precedence chain.
#[test]
fn resolves_each_property_from_its_highest_precedence_source() {
    let root = temporary_dir();
    let system = root.join("system");
    let user = root.join("user");
    let project = root.join("project");
    let nested = project.join("src");
    std::fs::create_dir_all(&nested).expect("create nested project path");
    let system_file = write_config(
        &system,
        "config.yaml",
        "provider: system\nmodel: system-model\nno_stream: false\n",
    );
    let user_file = write_config(
        &user,
        "config.json",
        r#"{"provider":"user","format":"json"}"#,
    );
    let project_file = write_config(
        &project,
        "config.yml",
        "provider: project\nmodel: project-model\n",
    );
    let environment = BTreeMap::from([
        ("SRIG_PROVIDER".to_owned(), "environment".to_owned()),
        ("SRIG_NO_STREAM".to_owned(), "true".to_owned()),
    ]);
    let resolver = ConfigResolver::with_inputs(Some(system), Some(user), nested, environment);
    let cli = BTreeMap::from([("provider".to_owned(), serde_json::json!("cli"))]);

    let result = resolver.resolve(&cli).expect("resolved config");
    assert_eq!(result.entries["provider"].value, "cli");
    assert_eq!(result.entries["model"].value, "project-model");
    assert_eq!(result.entries["format"].value, "json");
    assert_eq!(result.entries["no_stream"].value, true);
    assert_eq!(
        result.entries["provider"].source,
        ConfigSource::CommandLine("--provider".to_owned())
    );
    assert_eq!(
        result.entries["model"].source,
        ConfigSource::File(project_file)
    );
    assert_eq!(
        result.entries["format"].source,
        ConfigSource::File(user_file)
    );
    assert_eq!(
        result.entries["no_stream"].source,
        ConfigSource::Environment("SRIG_NO_STREAM".to_owned())
    );
    assert!(system_file.exists());
    std::fs::remove_dir_all(root).expect("remove temporary directory");
}

/// Verifies the nearest project config and supported extension order.
#[test]
fn selects_yaml_before_json_and_nearest_ancestor_project_file() {
    let root = temporary_dir();
    let outer = root.join("outer");
    let inner = outer.join("inner");
    std::fs::create_dir_all(&inner).expect("create nested path");
    write_config(&outer, "config.json", r#"{"model":"outer"}"#);
    let chosen = write_config(&inner, "config.yaml", "model: inner\n");
    write_config(&inner, "config.json", r#"{"model":"ignored"}"#);
    let resolver = ConfigResolver::with_inputs(None, None, inner.clone(), BTreeMap::new());

    let result = resolver.resolve(&BTreeMap::new()).expect("resolved config");
    assert_eq!(result.entries["model"].value, "inner");
    assert_eq!(result.entries["model"].source, ConfigSource::File(chosen));
    std::fs::remove_dir_all(root).expect("remove temporary directory");
}

/// Verifies malformed discovered files produce actionable errors.
#[test]
fn rejects_malformed_configuration_file() {
    let root = temporary_dir();
    write_config(&root, "config.json", "{not-json}");
    let resolver = ConfigResolver::with_inputs(None, None, root.clone(), BTreeMap::new());

    let error = resolver
        .resolve(&BTreeMap::new())
        .expect_err("invalid JSON");
    assert!(error.message.contains("invalid configuration file"));
    std::fs::remove_dir_all(root).expect("remove temporary directory");
}

/// Verifies wrong property types are rejected at the file boundary.
#[test]
fn rejects_invalid_property_type() {
    let root = temporary_dir();
    write_config(&root, "config.yaml", "no_stream: maybe\n");
    let resolver = ConfigResolver::with_inputs(None, None, root.clone(), BTreeMap::new());

    let error = resolver
        .resolve(&BTreeMap::new())
        .expect_err("invalid property type");
    assert!(error.message.contains("invalid value type"));
    std::fs::remove_dir_all(root).expect("remove temporary directory");
}

/// Verifies unsupported format names are rejected during resolution.
#[test]
fn rejects_invalid_format_value() {
    let root = temporary_dir();
    write_config(&root, "config.json", r#"{"format":"yaml"}"#);
    let config_resolver = ConfigResolver::with_inputs(None, None, root.clone(), BTreeMap::new());

    let error = config_resolver
        .resolve(&BTreeMap::new())
        .expect_err("unsupported format");
    assert!(error.message.contains("format` must be human or json"));
    std::fs::remove_dir_all(root).expect("remove temporary directory");
}

/// Verifies inspection output includes values and source designators.
#[test]
fn renders_effective_values_with_sources() {
    let root = temporary_dir();
    let path = write_config(&root, "config.json", r#"{"provider":"local"}"#);
    let config_resolver = ConfigResolver::with_inputs(None, None, root.clone(), BTreeMap::new());
    let effective_config = config_resolver
        .resolve(&BTreeMap::new())
        .expect("resolved config");

    let human = render(&effective_config, false).expect("human output");
    let json = render(&effective_config, true).expect("JSON output");
    assert!(human.contains("provider = \"local\""));
    assert!(human.contains(&path.display().to_string()));
    assert!(json.contains("\"property\": \"provider\""));
    assert!(json.contains("config.json"));
    std::fs::remove_dir_all(root).expect("remove temporary directory");
}

/// Verifies provider credential contents never enter displayed configuration.
#[test]
fn does_not_display_provider_credential_contents() {
    let root = temporary_dir();
    let environment = BTreeMap::from([
        ("SRIG_API_KEY_ENV".to_owned(), "PROVIDER_SECRET".to_owned()),
        ("PROVIDER_SECRET".to_owned(), "private-token".to_owned()),
    ]);
    let config_resolver = ConfigResolver::with_inputs(None, None, root.clone(), environment);
    let effective_config = config_resolver
        .resolve(&BTreeMap::new())
        .expect("resolved config");

    let rendered = render(&effective_config, true).expect("JSON output");
    assert!(rendered.contains("PROVIDER_SECRET"));
    assert!(!rendered.contains("private-token"));
    std::fs::remove_dir_all(root).expect("remove temporary directory");
}
