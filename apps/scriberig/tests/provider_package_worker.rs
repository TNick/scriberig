//! Exercises real provider package load, generation refresh, and invocation.

#![cfg(feature = "isolated-plugin-worker")]

#[path = "provider_package_worker/catalog_assertions.rs"]
mod catalog_assertions;
#[path = "provider_package_worker/one_shot.rs"]
mod one_shot;
#[path = "provider_package_worker/support.rs"]
mod support;
use support::{
    allowlisted_provider, assert_provider_turn, assert_refresh_registrations,
    provider_plugin_artifact, start_fixture_worker, start_fixture_worker_with_commands,
    start_fixture_worker_with_policy,
};
#[path = "provider_package_worker/brokered_http.rs"]
mod brokered_http;
#[path = "provider_package_worker/semantic_lifecycle.rs"]
mod semantic_lifecycle;
#[path = "provider_package_worker/semantic_operation.rs"]
mod semantic_operation;

use catalog_assertions::{assert_single_fixture_catalog_model, catalog_model};

use std::collections::BTreeSet;
use std::sync::Arc;

use rpi_extensions::GenerationCandidate;
use scriberig_agent_runtime::plugin_worker::{PluginSemanticDispatcher, sandboxed_launcher};
use scriberig_app::ProviderExtensionRuntime;
use serde_json::Value;

/// Refuses host semantic operations for the provider-only worker fixture.
struct DenyOperations;

impl PluginSemanticDispatcher for DenyOperations {
    /// Denies every semantic operation requested by a provider-only fixture.
    ///
    /// # Parameters
    /// - `operation`: Requested host semantic operation.
    /// - `_arguments`: Operation arguments, which this dispatcher does not inspect.
    ///
    /// # Returns
    /// No successful operation result.
    ///
    /// # Errors
    /// Always returns a denial.
    fn dispatch(&self, operation: &str, _arguments: &Value) -> Result<Value, String> {
        Err(format!("provider-only fixture cannot call `{operation}`"))
    }
}

/// Loads a provider package, refreshes its generation, and invokes both adapters.
#[tokio::test]
async fn registered_provider_package_completes_turns_across_generation_refresh() {
    let launcher = sandboxed_launcher()
        .expect("positive provider-package test requires a verified sandbox launcher");
    let plugin = provider_plugin_artifact();
    let protected_data = tempfile::tempdir().expect("protected fixture directory");
    let protected_roots = vec![
        protected_data
            .path()
            .canonicalize()
            .expect("canonical protected fixture directory"),
    ];
    let runtime = ProviderExtensionRuntime::new();

    let allowed_tools = BTreeSet::from(["scriberig_fixture_tool".to_owned()]);
    let allowed_commands = BTreeSet::from(["scriberig-fixture-command".to_owned()]);
    let first_worker = start_fixture_worker_with_commands(
        &plugin,
        &protected_roots,
        launcher,
        allowed_tools.clone(),
        allowed_commands.clone(),
    );
    assert_refresh_registrations(&first_worker);
    let first_providers = first_worker
        .provider_adapters(&allowlisted_provider())
        .expect("read initial provider registration");
    assert_eq!(first_providers.len(), 1);
    let retired_provider = Arc::clone(&first_providers[0]);
    let initial_models = retired_provider.models().to_vec();
    assert_eq!(initial_models.len(), 1);
    assert_eq!(initial_models[0].id, "scriberig-fixture-model");
    let initial_generation = runtime.extension_catalog_snapshot();
    let initial_candidate = GenerationCandidate::from_generation(&initial_generation.extension)
        .retain(Arc::clone(&first_worker) as Arc<dyn std::any::Any + Send + Sync>);
    runtime
        .refresh_provider_extension_generation(|| {
            Ok::<_, String>((initial_candidate, initial_models, first_providers))
        })
        .expect("publish provider and model catalog together");
    assert_single_fixture_catalog_model(&runtime);
    let first_model = catalog_model(&runtime);
    assert_provider_turn(&*retired_provider, &first_model).await;

    let second_worker = start_fixture_worker_with_commands(
        &plugin,
        &protected_roots,
        launcher,
        allowed_tools,
        allowed_commands,
    );
    assert_refresh_registrations(&second_worker);
    let second_providers = second_worker
        .provider_adapters(&allowlisted_provider())
        .expect("read refreshed provider registration");
    let second_models = second_providers[0].models().to_vec();
    let before_refresh = runtime.extension_catalog_snapshot();
    let second_candidate = GenerationCandidate::from_generation(&before_refresh.extension)
        .retain(Arc::clone(&second_worker) as Arc<dyn std::any::Any + Send + Sync>);
    runtime
        .refresh_provider_extension_generation(|| {
            Ok::<_, String>((second_candidate, second_models, second_providers))
        })
        .expect("publish refreshed provider and catalog generation");
    let after_refresh = runtime.extension_catalog_snapshot();
    assert!(after_refresh.extension.id() > before_refresh.extension.id());
    assert_eq!(
        after_refresh.extension.catalog_revision(),
        Some(after_refresh.catalog.revision)
    );
    assert_single_fixture_catalog_model(&runtime);

    assert_provider_turn(&*retired_provider, &first_model).await;
    let current_provider = runtime
        .provider("scriberig-fixture-provider")
        .expect("resolve provider from refreshed generation");
    assert_provider_turn(&*current_provider, &catalog_model(&runtime)).await;
}

/// Executes an allowlisted tool registered by the real fixture package.
#[test]
fn registered_package_tool_executes_only_when_allowlisted() {
    let launcher = sandboxed_launcher().expect("verified sandbox launcher");
    let plugin = provider_plugin_artifact();
    let protected_data = tempfile::tempdir().expect("protected fixture directory");
    let protected_roots = vec![
        protected_data
            .path()
            .canonicalize()
            .expect("canonical root"),
    ];
    let worker = start_fixture_worker(
        &plugin,
        &protected_roots,
        launcher,
        BTreeSet::from(["scriberig_fixture_tool".to_owned()]),
    );
    assert!(worker.advertises("scriberig_fixture_tool"));
    let result = worker
        .call_tool("scriberig_fixture_tool", serde_json::json!({}))
        .expect("authorized package tool call");
    assert!(
        result
            .to_string()
            .contains("isolated fixture tool completed")
    );
    assert!(
        worker
            .call_tool("unlisted_fixture_tool", serde_json::json!({}))
            .is_err()
    );
}

/// Invokes a registered package command only when the host allowlists it.
#[test]
fn registered_package_command_executes_only_when_allowlisted() {
    let launcher = sandboxed_launcher().expect("verified sandbox launcher");
    let plugin = provider_plugin_artifact();
    let protected_data = tempfile::tempdir().expect("protected fixture directory");
    let protected_roots = vec![
        protected_data
            .path()
            .canonicalize()
            .expect("canonical root"),
    ];
    let worker = start_fixture_worker_with_commands(
        &plugin,
        &protected_roots,
        launcher,
        BTreeSet::new(),
        BTreeSet::from(["scriberig-fixture-command".to_owned()]),
    );
    let commands = worker
        .advertised_commands()
        .expect("read allowlisted package commands");
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].name, "scriberig-fixture-command");
    let response = worker
        .call_command("scriberig-fixture-command", "hello")
        .expect("invoke allowlisted package command");
    assert_eq!(response["kind"], "message");
    assert_eq!(response["text"], "fixture command received: hello");
    let response = worker
        .call_command_action(
            "scriberig-fixture-command",
            "hello",
            Some(serde_json::json!({"value": "selected-option"})),
        )
        .expect("continue package command after selector input");
    assert_eq!(
        response["text"],
        "fixture command action received: selected-option"
    );
    assert!(worker.call_command("unlisted-command", "").is_err());
}
