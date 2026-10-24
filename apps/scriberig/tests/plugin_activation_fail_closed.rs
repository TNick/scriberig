//! Proves native plugin and package roots stay rejected before any provider runs.
//!
//! Task 144 G10 and task 149 §§4 and 12 keep native extension activation
//! deny-by-default until an upstream isolated worker is composed into the branch
//! tool catalog. This target runs the real executable so the fail-closed gate is
//! verified on the product path instead of by source inspection: every
//! configured native-code input must stop the one-shot agent before provider
//! setup, and a clean environment must still reach the deterministic Echo
//! provider.

use std::path::Path;
use std::process::Output;

/// Environment variables that request native extension or package loading.
const NATIVE_CODE_INPUTS: [&str; 3] = [
    "SCRIBE_AGENT_PACKAGE_ROOTS",
    "SCRIBE_AGENT_EXTENSION_DIRS",
    "SCRIBE_AGENT_PLUGIN_FILES",
];

/// Configuration properties that could change the deterministic one-shot
/// outcome, such as replacing the fallback provider or suppressing the streamed
/// output line the clean-run assertion reads.
const PROVIDER_PROPERTIES: [&str; 8] = [
    "base_url",
    "model",
    "provider",
    "credential",
    "api_key_env",
    "db",
    "format",
    "no_stream",
];

/// Builds one `srig agent` command isolated from the caller's configuration.
///
/// - `root`: Temporary directory holding the SQLite store and config home.
///
/// # Returns
/// A command that reads no host model configuration or native-code request.
fn agent_command(root: &Path) -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_srig"));
    command
        .current_dir(root)
        .arg("agent")
        .arg("--db")
        .arg(root.join("scriberig.sqlite"))
        .arg("Reply with OK");
    // A host model config or credential must not turn the deterministic Echo
    // fallback into a real provider request.
    command
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env_remove("APPDATA")
        .env_remove("PROGRAMDATA");
    for name in NATIVE_CODE_INPUTS {
        command.env_remove(name);
    }
    for property in PROVIDER_PROPERTIES {
        command.env_remove(format!("SRIG_{}", property.to_uppercase()));
    }
    command
}

/// Runs the one-shot agent command against one temporary store.
///
/// - `root`: Temporary directory holding the SQLite store.
/// - `variable`: Optional environment variable requesting native code.
///
/// # Returns
/// The captured process output for the requested command.
fn run_one_shot(root: &Path, variable: Option<&str>) -> Output {
    let mut command = agent_command(root);
    if let Some(name) = variable {
        command.env(name, root.join("configured-native-code"));
    }
    command.output().expect("one-shot agent run")
}

/// Returns the combined standard output and standard error of one run.
///
/// - `output`: Captured process output.
///
/// # Returns
/// The process text used for the fail-closed assertions.
fn combined_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
}

/// Returns the rejection the active build reports for configured native code.
///
/// # Returns
/// The fail-closed message emitted when extension or plugin paths are set.
fn expected_plugin_rejection() -> &'static str {
    if cfg!(feature = "isolated-plugin-worker") {
        "isolated plugin worker composition is not activated"
    } else {
        "isolated plugin support is not enabled in this build"
    }
}

/// Verifies a configured package root stops the agent before provider setup.
#[test]
fn configured_package_root_stops_the_one_shot_agent() {
    let directory = tempfile::tempdir().expect("temporary project");
    let output = run_one_shot(directory.path(), Some("SCRIBE_AGENT_PACKAGE_ROOTS"));
    let text = combined_text(&output);
    assert!(
        !output.status.success(),
        "package root was accepted: {text}"
    );
    assert!(
        text.contains("package roots are unsupported"),
        "package root rejection is missing: {text}"
    );
    assert!(
        !text.contains("agent output"),
        "a provider ran despite the package root: {text}"
    );
}

/// Verifies configured extension directories and plugin files are rejected.
#[test]
fn configured_native_extension_inputs_stop_the_one_shot_agent() {
    let expected = expected_plugin_rejection();
    for variable in ["SCRIBE_AGENT_EXTENSION_DIRS", "SCRIBE_AGENT_PLUGIN_FILES"] {
        let directory = tempfile::tempdir().expect("temporary project");
        let output = run_one_shot(directory.path(), Some(variable));
        let text = combined_text(&output);
        assert!(!output.status.success(), "{variable} was accepted: {text}");
        assert!(
            text.contains(expected),
            "{variable} did not report the fail-closed rejection {expected}: {text}"
        );
        assert!(
            !text.contains("agent output"),
            "a provider ran despite {variable}: {text}"
        );
    }
}

/// Seeds one upstream-style project package registration beneath the project.
///
/// The layout and `packages.json` record shape match the project package
/// registry that the pinned upstream `rpi-extensions` package library owns, and
/// the artifact is a planted file that no loader can map. The point of the
/// registration is that a real upstream registry entry exists on disk while
/// `ScribeRig` runs, so a package-directory scan or auto-discovery path would
/// reach it and report or fail instead of running the deterministic provider.
///
/// - `root`: Temporary project root that holds `.rpi`.
///
/// # Panics
/// Panics when the registry or planted artifact cannot be written.
fn seed_upstream_project_package(root: &Path) {
    let package_dir = root.join(".rpi/packages/demo-provider/extensions");
    std::fs::create_dir_all(&package_dir).expect("upstream package directory");
    std::fs::write(
        root.join(".rpi/packages.json"),
        r#"{"packages":[{"name":"demo-provider","version":"0.1.0","source":"local","cargo_package":"demo-provider","artifacts":["libdemo_provider.so"]}]}"#,
    )
    .expect("upstream package registry");
    std::fs::write(
        package_dir.join("libdemo_provider.so"),
        b"not a loadable artifact",
    )
    .expect("planted package artifact");
}

/// Verifies an upstream-registered project package is neither discovered nor
/// loaded.
///
/// Task 145 §9 assigns package discovery to the upstream registry and artifact
/// APIs, and the product gate stays deny-by-default because no upstream API yet
/// replaces a live extension session and its model-catalog rows as one
/// generation, so discovery must stay absent even when the working project
/// already carries a well-formed upstream registration whose planted artifact no
/// loader could map. The run is judged on its exit status and the durable
/// operation/session lines because the streamed `agent output:` delta line is not
/// emitted on every stream settlement.
#[test]
fn an_upstream_registered_package_is_not_discovered() {
    let directory = tempfile::tempdir().expect("temporary project");
    seed_upstream_project_package(directory.path());
    let output = run_one_shot(directory.path(), None);
    let text = combined_text(&output);
    assert!(
        output.status.success(),
        "the upstream package registration failed the run: {text}"
    );
    assert!(
        text.contains("operation:") && text.contains("session:"),
        "the one-shot run recorded no operation or session: {text}"
    );
    assert!(
        !text.contains("package roots are unsupported")
            && !text.contains(expected_plugin_rejection()),
        "the plugin gate fired without configured native code: {text}"
    );
    let lowered = text.to_lowercase();
    for marker in ["plugin", "extension", "package", ".so"] {
        assert!(
            !lowered.contains(marker),
            "the run reported {marker} about the upstream-registered package: {text}"
        );
    }
}

/// Verifies the rejection is specific to configured native code.
#[test]
fn a_clean_environment_still_reaches_the_deterministic_agent() {
    let directory = tempfile::tempdir().expect("temporary project");
    let output = run_one_shot(directory.path(), None);
    let text = combined_text(&output);
    assert!(output.status.success(), "clean run failed: {text}");
    assert!(
        text.contains("agent output"),
        "the deterministic agent did not run: {text}"
    );
    assert!(
        !text.contains("package roots are unsupported")
            && !text.contains(expected_plugin_rejection()),
        "the plugin gate fired without configured native code: {text}"
    );
}
