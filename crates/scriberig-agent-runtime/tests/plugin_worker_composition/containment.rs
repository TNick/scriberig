//! Credential and protected-store containment for one plugin worker process.
//!
//! The worker must never receive the ambient host environment, a canonical-store
//! path, or any declared value that names protected `ScribeRig` state. A launch
//! that could reach such state is refused before a process starts, and the
//! settled echo call reports only what the admitted worker actually received.

use std::collections::BTreeSet;
use std::sync::Arc;

use rpi_agent::TextContentOrImage;
use rpi_extensions::isolated::WorkerSandboxPolicy;
use scriberig_agent_runtime::plugin_worker::PluginToolBridge;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::fixture::{ECHO_TOOL, FixtureRoot, PLUGIN_CAPABILITY, launch_spec, start_worker};
use super::helpers::{execute, granted, request, start_with};

/// Verifies a read root that covers the protected store is refused.
#[test]
fn a_read_root_covering_protected_state_is_refused() {
    let root = FixtureRoot::new("containment-root");
    let mut covering = launch_spec(&[ECHO_TOOL], &root, None);
    covering.sandbox = WorkerSandboxPolicy::sandboxed(
        vec![root.root.clone(), FixtureRoot::executable_dir()],
        Vec::new(),
    );

    let refused = start_with(&covering, &root);
    let Err(denial) = refused else {
        panic!("a read root that covers the canonical store must be refused");
    };
    assert!(
        denial
            .to_string()
            .contains("would expose protected ScribeRig state"),
        "a covering read root did not fail closed: {denial}"
    );
    assert!(
        std::fs::read_dir(&root.plugin_dir)
            .expect("plugin directory")
            .next()
            .is_none(),
        "a refused start must not leave any worker-side state"
    );

    root.remove();
}

/// Verifies a declared environment value or an inherited host environment that
/// exposes protected state or ambient credentials is refused.
#[test]
fn an_exposed_environment_is_refused() {
    let root = FixtureRoot::new("containment-env");
    let declared = launch_spec(&[ECHO_TOOL], &root, None);

    let mut leaking = declared.clone();
    leaking.environment.insert(
        "SCRIG_FIXTURE_STORE".to_owned(),
        root.store_dir
            .join("scriberig.sqlite")
            .display()
            .to_string(),
    );
    let refused_value = start_with(&leaking, &root);
    let Err(leaking) = refused_value else {
        panic!("a protected path in the environment must be refused");
    };
    assert!(
        leaking
            .to_string()
            .contains("exposes protected ScribeRig state"),
        "a leaking environment value did not fail closed: {leaking}"
    );

    let mut inherited = declared;
    inherited.inherit_environment = true;
    let refused_inheritance = start_with(&inherited, &root);
    let Err(inherited) = refused_inheritance else {
        panic!("an inherited host environment must be refused");
    };
    assert!(
        inherited
            .to_string()
            .contains("must not inherit the host environment"),
        "an inherited environment did not fail closed: {inherited}"
    );
    assert!(
        std::fs::read_dir(&root.plugin_dir)
            .expect("plugin directory")
            .next()
            .is_none(),
        "a refused start must not leave any worker-side state"
    );

    root.remove();
}

/// Verifies an admitted worker's provider-visible definitions and settled call
/// carry no protected store path and no ambient host environment entry.
#[test]
fn an_admitted_worker_reveals_no_protected_state() {
    let root = FixtureRoot::new("containment-call");
    let worker = Arc::new(
        start_worker(&[ECHO_TOOL], &[ECHO_TOOL], &root, None, true)
            .expect("authorized fixture worker"),
    );
    let bridge =
        PluginToolBridge::new(Arc::clone(&worker), PLUGIN_CAPABILITY).expect("attested bridge");
    let composed = bridge.compose(request(granted())).expect("compose");
    let store_path = root.store_dir.display().to_string();

    let definitions: Vec<String> = composed
        .catalog()
        .definitions()
        .iter()
        .map(|definition| serde_json::to_string(definition).unwrap_or_default())
        .collect();
    assert!(
        definitions
            .iter()
            .all(|definition| !definition.contains(&store_path)),
        "a protected path entered the provider-visible tool definitions"
    );

    let result = execute(
        &composed.tools()[0],
        json!({"probe": "env"}),
        CancellationToken::new(),
    )
    .expect("echo call settles");
    let TextContentOrImage::Text(text) = &result.content[0] else {
        panic!("the provider projection must be text");
    };
    let echoed: Value = serde_json::from_str(&text.text).expect("echoed worker report");
    let received = echoed["environment"]
        .as_object()
        .expect("worker environment report");
    let host_names: BTreeSet<String> = std::env::vars_os()
        .map(|(name, _)| name.to_string_lossy().into_owned())
        .collect();
    let shared: Vec<&String> = received
        .keys()
        .filter(|name| host_names.contains(*name))
        .collect();

    assert!(
        shared.is_empty(),
        "the worker received ambient host environment entries: {shared:?}"
    );
    assert!(
        received.contains_key("SCRIG_FIXTURE_TOOLS"),
        "the declared worker environment must still reach the worker"
    );
    assert!(
        !text.text.contains(&store_path),
        "a protected store path reached the provider-visible result"
    );

    root.remove();
}
