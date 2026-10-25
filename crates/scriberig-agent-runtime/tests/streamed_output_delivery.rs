//! Proves every streamed assistant delta reaches the host text observer.
//!
//! The one-shot CLI and the TUI render live provider text through the
//! `OutputCallback` handed to `run_upstream_branch_invocation`. The pinned faux
//! provider can emit its last deltas and settle the loop inside one runtime
//! poll, so the invocation's subscription loop is free to settle while those
//! deltas are still buffered in the upstream event broadcast. These runs pin
//! that a settled loop still forwards every buffered delta: the product
//! `srig agent` streamed `agent output:` line, and the fail-closed plugin gate
//! that asserts that line is absent when native code is configured, both depend
//! on the streamed text arriving exactly once and in order.

use std::sync::{Arc, Mutex};

use rpi_ai::providers::faux::{FauxProvider, FauxScript};
use rpi_ai::{Api, Model, Provider};
use scriberig_agent_runtime::context::{BranchExecutionContext, ContextCompiler, ContextPolicy};
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchInvocation, run_upstream_branch_invocation,
};
use scriberig_core::EntityId;
use tokio_util::sync::CancellationToken;

/// Assistant text the faux provider streams as several deltas per settlement.
const STREAMED_TEXT: &str =
    "No provider endpoint configured. The deterministic fallback answers instead.";

/// Number of consecutive settlements checked for a lost delta. The defect is a
/// settlement race, so repeated settlements are what make the regression fail.
const SETTLEMENTS: usize = 96;

/// Builds the deterministic model row for one settlement.
///
/// # Returns
/// An upstream model descriptor matching the product Echo fallback shape.
fn streamed_model() -> Model {
    Model::new(
        "echo-stream",
        "ScribeRig Echo",
        Api::Faux,
        "scriberig-echo",
        "local://echo",
    )
}

/// Builds one no-tool branch invocation scripted with `STREAMED_TEXT`.
///
/// # Returns
/// The invocation bound to the deterministic fallback provider.
fn invocation() -> UpstreamBranchInvocation {
    let model = streamed_model();
    let provider: Arc<dyn Provider> = FauxProvider::with_models(
        FauxScript::new().with_text(STREAMED_TEXT),
        vec![model.clone()],
    );
    let context =
        ContextCompiler::new(ContextPolicy::default()).compile(&BranchExecutionContext::new(
            "stream-session",
            "stream-branch",
            "stream settlement proof",
            "Reply with OK",
        ));
    UpstreamBranchInvocation {
        branch: EntityId::from_bytes([92; 16]),
        provider,
        model,
        request_secret: None,
        bearer: false,
        applied_settings: None,
        provider_profile: None,
        thinking_level: rpi_ai::ThinkingLevel::Off,
        context,
        tools: Vec::new(),
        activity: Arc::new(scriberig_agent_runtime::orchestration::ActivitySequence::default()),
    }
}

/// Settles one branch invocation and collects the streamed fragments.
///
/// # Returns
/// The fragments the output observer received in arrival order and the durable
/// final assistant text.
///
/// # Panics
/// Panics when the invocation executor cannot be built or the run fails.
fn settle() -> (Vec<String>, String) {
    let invocation = invocation();
    let fragments = Arc::new(Mutex::new(Vec::new()));
    let observer = {
        let fragments = Arc::clone(&fragments);
        Arc::new(move |text: &str| {
            fragments
                .lock()
                .expect("fragment sink")
                .push(text.to_owned());
        }) as Arc<dyn Fn(&str) + Send + Sync>
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("stream settlement runtime");
    let result = runtime
        .block_on(run_upstream_branch_invocation(
            invocation,
            "Reply with OK",
            CancellationToken::new(),
            Some(observer),
        ))
        .expect("stream settlement run");
    let fragments = fragments
        .lock()
        .expect("fragment sink")
        .iter()
        .cloned()
        .collect();
    (fragments, result.output)
}

/// Verifies each settlement delivers the complete streamed text exactly once.
#[test]
fn every_settlement_forwards_each_streamed_delta() {
    for settlement in 0..SETTLEMENTS {
        let (fragments, output) = settle();
        assert_eq!(output, STREAMED_TEXT, "settlement {settlement} output");
        assert!(
            fragments.len() > 1,
            "settlement {settlement} streamed {fragments:?} instead of several deltas"
        );
        assert_eq!(
            fragments.concat(),
            STREAMED_TEXT,
            "settlement {settlement} lost or reordered a streamed delta: {fragments:?}"
        );
    }
}
