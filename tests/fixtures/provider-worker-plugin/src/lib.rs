//! Provides a real native provider plugin for isolated worker integration tests.

#![allow(missing_docs, reason = "the export macro generates the ABI entrypoint")]
#![allow(
    unsafe_code,
    reason = "this ABI fixture must exercise ownership transfer for native tool callbacks"
)]
#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "the export macro validates the host-provided ABI pointer before dereferencing"
)]

use rpi_ai::{Api, Model};
use rpi_plugin_sdk::callbacks::free_string;
use rpi_plugin_sdk::{RuntimeActionFn, StableToolSchema, StbString, StbStringRef};
use std::sync::OnceLock;

mod provider_callbacks;
mod tool_callbacks;
use provider_callbacks::{brokered_provider_request, provider_request};
use tool_callbacks::{
    fixture_command, fixture_tool_cancel, fixture_tool_destroy, fixture_tool_execute,
    fixture_tool_poll,
};

const PROVIDER_ID: &str = "scriberig-fixture-provider";
const MODEL_ID: &str = "scriberig-fixture-model";
const TOOL_NAME: &str = "scriberig_fixture_tool";
const COMMAND_NAME: &str = "scriberig-fixture-command";
const SEMANTIC_REQUEST_PREFIX: &str = "fixture semantic rename: ";
const BROKER_PROVIDER_ID: &str = "scriberig-fixture-broker-provider";
const BROKER_MODEL_ID: &str = "scriberig-fixture-broker-model";
const BROKER_ENDPOINT_ENV: &str = "SCRIBE_FIXTURE_HTTP_ENDPOINT";
static RUNTIME_ACTION: OnceLock<RuntimeActionFn> = OnceLock::new();

rpi_plugin_sdk::export_plugin!(|api| {
    let _ = RUNTIME_ACTION.set(api.runtime_action);
    eprintln!("fixture worker diagnostic: provider and tool registered");
    let model = Model::new(
        MODEL_ID,
        "ScribeRig Isolated Fixture",
        Api::Faux,
        PROVIDER_ID,
        "https://fixture.example.test/v1",
    );
    let broker_endpoint = std::env::var(BROKER_ENDPOINT_ENV).ok();
    let mut providers = vec![serde_json::json!({"id": PROVIDER_ID, "models": [model]})];
    if let Some(endpoint) = broker_endpoint.as_ref() {
        let broker_model = Model::new(
            BROKER_MODEL_ID,
            "ScribeRig Broker Fixture",
            Api::OpenaiCompletions,
            BROKER_PROVIDER_ID,
            format!("{endpoint}/v1"),
        );
        providers.push(serde_json::json!({
            "id": BROKER_PROVIDER_ID,
            "models": [broker_model]
        }));
    }
    let manifest = serde_json::json!({"providers": providers});
    let Some(declare) = api.declare else {
        return 1;
    };
    if declare(StbStringRef::from_str(&manifest.to_string())) != 0 {
        return 2;
    }
    let Some(register_provider) = api.register_provider else {
        return 3;
    };
    if register_provider(
        StbStringRef::from_str(PROVIDER_ID),
        StbStringRef::from_str("https://fixture.example.test/v1"),
        StbStringRef::from_str("faux"),
        provider_request,
        free_string,
        std::ptr::null_mut(),
    ) != 0
    {
        return 4;
    }
    if let Some(endpoint) = broker_endpoint {
        let Some(register_protected_provider) = api.register_protected_provider else {
            return 8;
        };
        if register_protected_provider(
            StbStringRef::from_str(BROKER_PROVIDER_ID),
            StbStringRef::from_str(&format!("{endpoint}/v1")),
            StbStringRef::from_str("openai-completions"),
            brokered_provider_request,
            free_string,
            std::ptr::null_mut(),
        ) != 0
        {
            return 9;
        }
    }
    let Some(register_tool) = api.register_tool else {
        return 5;
    };
    let schema = StableToolSchema {
        name: StbString::from_string(TOOL_NAME.to_owned()),
        description: StbString::from_string(
            "Runs the deterministic ScribeRig fixture tool.".to_owned(),
        ),
        parameters: StbString::from_string(
            r#"{"type":"object","properties":{"hostCapability":{"type":"object","properties":{"name":{"type":"string"},"arguments":{"type":"object"}},"required":["name"]}}}"#.to_owned(),
        ),
    };
    if register_tool(
        &raw const schema,
        fixture_tool_execute,
        fixture_tool_poll,
        fixture_tool_cancel,
        fixture_tool_destroy,
        free_string,
    ) != 0
    {
        return 7;
    }
    let Some(register_command) = api.register_command else {
        return 6;
    };
    register_command(
        StbStringRef::from_str(COMMAND_NAME),
        StbStringRef::from_str("Runs the isolated ScribeRig fixture command."),
        fixture_command,
    )
});
