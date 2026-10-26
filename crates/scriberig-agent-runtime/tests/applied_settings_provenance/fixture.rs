//! Provides the deterministic applied-settings fixture used by the provenance tests.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use rpi_ai::types::Api;
use rpi_ai::{AppliedRequestSettings, Context, Model, Provider, SimpleStreamOptions};
use scriberig_agent_runtime::context::{BranchExecutionContext, ContextCompiler, ContextPolicy};
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchInvocation, run_upstream_branch_invocation_blocking,
};
use scriberig_core::EntityId;

#[path = "fixture/mode.rs"]
mod mode;
#[path = "fixture/report.rs"]
mod report;
#[path = "fixture/stream.rs"]
mod stream;
pub(super) use mode::ReportMode;
use stream::completed_stream;

/// Credential text that must never enter an upstream applied-settings report.
pub(super) const CREDENTIAL_SENTINEL: &str = "applied-settings-credential-sentinel-2c7a";

/// Deterministic provider that publishes one applied-settings report per request.
pub(super) struct SettingsProvider {
    /// Stable fixture provider identity.
    provider_id: String,
    /// Exact model rows served by the fixture.
    served: Vec<Model>,
    /// Report the fixture publishes for each prepared request.
    mode: ReportMode,
}

impl SettingsProvider {
    /// Builds a fixture provider publishing the selected report shape.
    ///
    /// # Parameters
    /// - `provider_id`: Stable fixture provider identity.
    /// - `mode`: Report shape published after request preparation.
    /// - `served`: Model rows the fixture serves.
    ///
    /// # Returns
    /// A fixture provider bound to those model rows.
    pub(super) fn new(provider_id: &str, mode: ReportMode, served: Vec<Model>) -> Self {
        Self {
            provider_id: provider_id.to_owned(),
            served,
            mode,
        }
    }

    /// Publishes the configured report, when the mode publishes one.
    ///
    /// - `self`: Fixture provider holding the report mode.
    /// - `model`: Exact model selected by the upstream loop.
    /// - `callback`: Upstream sink for sanitized final-request settings.
    fn publish(&self, model: &Model, callback: &rpi_ai::AppliedRequestSettingsCallback) {
        if let Some(report) = self.report(model) {
            callback(report);
        }
    }

    /// Builds the report for the configured mode.
    ///
    /// - `self`: Fixture provider holding the report mode.
    /// - `model`: Exact model bound to the current request.
    ///
    /// # Returns
    /// The report to publish, or `None` for the silent mode.
    fn report(&self, model: &Model) -> Option<AppliedRequestSettings> {
        if matches!(self.mode, ReportMode::Silent) {
            return None;
        }
        let mut applied_values = BTreeMap::new();
        applied_values.insert("model".to_owned(), serde_json::json!(model.id));
        applied_values.insert("temperature".to_owned(), serde_json::json!(0.25));
        applied_values.insert("stream".to_owned(), serde_json::json!(true));
        applied_values.insert(
            "reasoning".to_owned(),
            serde_json::json!({ "effort": "medium" }),
        );
        Some(report::build(self.mode, model, applied_values))
    }
}

#[async_trait]
impl Provider for SettingsProvider {
    /// Returns the fixture provider identity.
    ///
    /// # Returns
    /// Stable fixture provider identifier.
    fn id(&self) -> &str {
        &self.provider_id
    }

    /// Returns the model rows served by the fixture.
    ///
    /// # Returns
    /// Exact fixture model rows.
    fn models(&self) -> &[Model] {
        &self.served
    }

    /// Streams a response for a request that carried no report sink.
    ///
    /// - `self`: Fixture provider holding identity and served models.
    /// - `model`: Exact model selected by the upstream loop.
    /// - `_ctx`: Provider context; the fixture answers deterministically.
    /// - `_opts`: Stream options, unused by the fixture.
    ///
    /// # Returns
    /// The completed response event stream.
    async fn stream_simple(
        &self,
        model: &Model,
        _ctx: &Context,
        _opts: &SimpleStreamOptions,
    ) -> rpi_ai::AssistantMessageEventStream {
        completed_stream(model, self.provider_id.as_str())
    }

    /// Streams a response and publishes the configured final-body report.
    ///
    /// - `self`: Fixture provider holding the report mode.
    /// - `model`: Exact model selected by the upstream loop.
    /// - `_ctx`: Provider context; the fixture answers deterministically.
    /// - `_opts`: Stream options, unused by the fixture.
    /// - `callback`: Upstream sink for sanitized final-request settings.
    ///
    /// # Returns
    /// The completed response event stream.
    async fn stream_simple_with_applied_settings(
        &self,
        model: &Model,
        _ctx: &Context,
        _opts: &SimpleStreamOptions,
        callback: rpi_ai::AppliedRequestSettingsCallback,
    ) -> rpi_ai::AssistantMessageEventStream {
        self.publish(model, &callback);
        completed_stream(model, self.provider_id.as_str())
    }
}

/// Builds one fixture model row carrying credential-bearing catalog metadata.
///
/// # Parameters
/// - `provider_id`: Provider identity owning the model.
/// - `model_id`: Model identity served by the fixture.
///
/// # Returns
/// An upstream model descriptor whose endpoint and headers carry the sentinel.
pub(super) fn fixture_model(provider_id: &str, model_id: &str) -> Model {
    let mut model = Model::new(
        model_id,
        "Applied Settings Fixture Model",
        Api::Other("applied-settings-fixture".to_owned()),
        provider_id,
        format!("https://provider.invalid/v1?token={CREDENTIAL_SENTINEL}"),
    );
    model.provider_options_schema = Some(rpi_ai::ProviderOptionsSchema {
        version: 1,
        schema: serde_json::json!({
            "type": "object",
            "properties": {
                "future_vendor_mode": {"type": "string", "enum": ["fast", "accurate"]}
            },
            "additionalProperties": false
        }),
    });
    model
        .provider_options
        .insert("future_vendor_mode".to_owned(), serde_json::json!("fast"));
    model.headers = Some(BTreeMap::from([(
        "authorization".to_owned(),
        format!("Bearer {CREDENTIAL_SENTINEL}"),
    )]));
    model
}

/// Builds the exact `ScribeRig` context compilation used by the fixture.
///
/// # Returns
/// The compiled branch context supplied to the upstream invocation.
pub(super) fn compiled_context() -> scriberig_agent_runtime::context::CompiledContext {
    ContextCompiler::new(ContextPolicy::default()).compile(&BranchExecutionContext::new(
        "applied-settings-session",
        "applied-settings-branch",
        "applied settings provenance proof",
        "review the bounded request",
    ))
}

/// Builds one branch invocation bound to the fixture provider.
///
/// # Parameters
/// - `provider`: Fixture provider under test.
/// - `model`: Exact model bound to the invocation.
/// - `applied_settings`: Optional host observer for validated reports.
///
/// # Returns
/// A branch invocation with no semantic tools.
pub(super) fn invocation(
    provider: Arc<dyn Provider>,
    model: Model,
    applied_settings: Option<rpi_ai::AppliedRequestSettingsCallback>,
) -> UpstreamBranchInvocation {
    UpstreamBranchInvocation {
        branch: EntityId::from_bytes([81; 16]),
        provider,
        model,
        request_secret: None,
        bearer: false,
        applied_settings,
        provider_profile: Some("host-provider-profile".to_owned()),
        thinking_level: rpi_ai::ThinkingLevel::Off,
        context: compiled_context(),
        tools: Vec::new(),
        activity: Arc::new(scriberig_agent_runtime::orchestration::ActivitySequence::default()),
        provider_lifecycle: None,
    }
}

/// Runs one fixture branch invocation and returns its captured requests.
///
/// # Parameters
/// - `mode`: Report shape published by the fixture provider.
///
/// # Returns
/// The captured safe request projections, in dispatch order.
///
/// # Errors
/// Panics when the deterministic upstream invocation fails.
pub(super) fn capture(mode: ReportMode) -> Vec<scriberig_session::ModelRequestProjection> {
    let provider_id = "fixture-applied-settings";
    let model = fixture_model(provider_id, "applied-settings-model");
    let provider = Arc::new(SettingsProvider::new(
        provider_id,
        mode,
        vec![model.clone()],
    ));
    run_upstream_branch_invocation_blocking(
        invocation(provider as Arc<dyn Provider>, model, None),
        "review the bounded request",
    )
    .expect("upstream branch invocation")
    .model_requests
}
