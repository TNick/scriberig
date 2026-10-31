//! Proves one-shot product provider provenance for an isolated package.

use super::provider_plugin_artifact;
use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;
use std::process::Command;

/// Verifies the ordinary one-shot product path selects an allowlisted package provider.
#[test]
fn one_shot_agent_activates_and_invokes_the_configured_provider_package() {
    let plugin = provider_plugin_artifact();
    let project = tempfile::tempdir().expect("isolated product project");
    let database = project.path().join("scriberig.sqlite");
    let extension_dir = project.path().join("extensions");
    std::fs::create_dir(&extension_dir).expect("provider extension directory");
    let staged_plugin = extension_dir.join(plugin.file_name().expect("plugin filename"));
    std::fs::copy(&plugin, &staged_plugin).expect("stage provider plugin artifact");
    let output = Command::new(env!("CARGO_BIN_EXE_srig"))
        .current_dir(project.path())
        .arg("agent")
        .arg("--db")
        .arg(&database)
        .arg("--provider")
        .arg("scriberig-fixture-provider")
        .arg("--model")
        .arg("scriberig-fixture-model")
        .arg("--no-stream")
        .arg("Return the configured package provider response.")
        .env("HOME", project.path())
        .env("XDG_CONFIG_HOME", project.path().join("config"))
        .env_remove("APPDATA")
        .env_remove("PROGRAMDATA")
        .env("SCRIBE_AGENT_EXTENSION_DIRS", &extension_dir)
        .env(
            "SCRIBE_AGENT_PLUGIN_PROVIDERS",
            "scriberig-fixture-provider",
        )
        .env("SCRIBE_AGENT_PLUGIN_WORKER", env!("CARGO_BIN_EXE_srig"))
        .env_remove("SCRIBE_AGENT_PACKAGE_ROOTS")
        .env("SCRIBE_AGENT_PLUGIN_CAPABILITY", "fixture-provider")
        .env("SCRIBE_AGENT_PLUGIN_TOOLS", "scriberig_fixture_tool")
        .env("RUST_LOG", "warn")
        .output()
        .expect("run one-shot product agent");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "one-shot package provider failed: {stdout}{stderr}"
    );
    assert!(
        stdout.contains("isolated fixture provider observed tool result"),
        "one-shot product did not return the package tool result: {stdout}{stderr}"
    );
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("fixture worker diagnostic: provider and tool registered"),
        "worker stderr was not forwarded through product logging: {combined}"
    );
    let session_id = diagnostic_identity(&stderr, "session:");
    let root_branch_id = diagnostic_identity(&stderr, "root branch:");
    let durable = SqliteSessionStore::open(&database)
        .expect("open CLI session database")
        .load(session_id)
        .expect("read CLI session graph")
        .expect("CLI session was persisted");
    let root = durable
        .branches
        .iter()
        .find(|branch| branch.id == root_branch_id)
        .expect("CLI root branch is durable");
    assert_eq!(root.model_invocations.len(), 2);
    assert!(root.model_invocations.iter().all(|invocation| {
        invocation.provider_id.as_deref() == Some("scriberig-fixture-provider")
            && invocation.model.as_deref() == Some("scriberig-fixture-model")
    }));
    assert!(
        root.model_invocations.iter().all(|invocation| {
            let Some(report) = invocation
                .request
                .as_ref()
                .and_then(|request| request.applied_settings.as_ref())
            else {
                return false;
            };
            report["complete"] == serde_json::json!(false)
                && report.get("requestEffectSha256").is_none()
                && report["omittedCategories"]
                    .as_array()
                    .is_some_and(|categories| {
                        categories
                            .iter()
                            .any(|category| category == "extension_managed_request_transforms")
                    })
        }),
        "isolated provider reports must remain incomplete in durable request provenance"
    );
    assert!(
        root.tool_invocations
            .iter()
            .any(|tool| tool.name == "scriberig_fixture_tool"),
        "the one-shot provider turn must settle its worker tool on the ScribeRig branch"
    );
}

/// Parses one stable identity from the one-shot CLI diagnostic record.
///
/// # Parameters
/// - `diagnostics`: Standard error emitted by the completed CLI process.
/// - `label`: Exact identity label to parse.
///
/// # Returns
/// The parsed `ScribeRig` entity identity.
fn diagnostic_identity(diagnostics: &str, label: &str) -> scriberig_core::EntityId {
    let value = diagnostics
        .lines()
        .find_map(|line| line.strip_prefix(label))
        .unwrap_or_else(|| panic!("CLI diagnostics lack {label:?}: {diagnostics}"))
        .trim();
    value
        .parse()
        .unwrap_or_else(|error| panic!("invalid {label:?} identity {value:?}: {error}"))
}
