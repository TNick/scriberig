//! Adapts `ScribeRig` semantic scenarios to upstream faux provider scripts.

use std::sync::Arc;

use rpi_ai::providers::faux::FauxScript;
use rpi_ai::providers::faux::{FauxProvider, FauxStep};
use rpi_ai::{Context, Model, Provider};

/// Identifies a deterministic `ScribeRig` product scenario.
#[derive(Clone)]
pub(super) enum NativeScenario {
    /// Project onboarding workflow using `ScribeRig` semantic operations.
    Bootstrap(crate::deterministic_provider::BootstrapProvider),
    /// Fixed arithmetic response used by the native smoke scenario.
    Arithmetic(crate::deterministic_provider::ArithmeticProvider),
    /// Post-manifest failure workflow used by project import coverage.
    BootstrapImportFailure(crate::bootstrap_failure_provider::BootstrapImportFailureProvider),
    /// PHP semantic self-hosting workflow and validation policy.
    SelfHostingPhp(crate::self_hosting_provider::PhpSelfHostingProvider),
    /// Fixed answer used when the one-shot command has no configured endpoint.
    Echo,
    /// Upstream-owned faux script used by deterministic runtime tests.
    #[cfg(test)]
    Faux(FauxScript),
    /// Continuation-capable fixture provider used by root continuation tests.
    #[cfg(test)]
    Continuation(
        std::sync::Arc<
            crate::native_tui_agent::continuation_scenario::ContinuationScenarioProvider,
        >,
    ),
}

impl NativeScenario {
    /// Builds a `rpi-ai` faux provider running this semantic scenario.
    ///
    /// The scenario only constructs an upstream assistant message. The faux
    /// provider owns stream settlement and the agent loop owns turn ordering.
    ///
    /// # Parameters
    /// - `model`: Exact upstream model assigned to the scenario.
    ///
    /// # Returns
    /// An upstream provider bound to the supplied model.
    pub(super) fn into_provider(self, model: Model) -> Arc<dyn Provider> {
        if matches!(&self, Self::Echo) {
            return FauxProvider::with_models(
                FauxScript::new().with_text("No provider endpoint configured."),
                vec![model],
            );
        }
        if matches!(&self, Self::Arithmetic(_)) {
            return FauxProvider::with_models(FauxScript::new().with_text("2"), vec![model]);
        }

        #[cfg(test)]
        if let Self::Faux(script) = self {
            return FauxProvider::with_models(script, vec![model]);
        }

        #[cfg(test)]
        if let Self::Continuation(provider) = self {
            return provider as std::sync::Arc<dyn Provider>;
        }

        let scenario = Arc::new(self);
        let scenario_model = model.clone();
        let script = FauxScript::new();
        // Each scenario step is context-driven, so keep a bounded pool of
        // factories available for the full tool loop instead of consuming a
        // single one-shot response.
        let steps = (0..256)
            .map(|_| {
                let scenario = Arc::clone(&scenario);
                let scenario_model = scenario_model.clone();
                FauxStep::factory(move |context| scenario.respond(context, &scenario_model))
            })
            .collect();
        script.set_responses(steps);
        FauxProvider::with_models(script, vec![model])
    }

    /// Produces one upstream assistant message from the current provider context.
    ///
    /// # Parameters
    /// - `self`: Selected semantic scenario.
    /// - `context`: Exact upstream context prepared by `rpi-agent`.
    /// - `model`: Exact upstream model assigned to the scenario.
    ///
    /// # Returns
    /// An upstream final, tool-call, or error message.
    fn respond(&self, context: &Context, model: &Model) -> rpi_ai::AssistantMessage {
        let result = match self {
            Self::Bootstrap(_) => {
                crate::deterministic_provider::BootstrapProvider::respond(context, model)
            }
            Self::Arithmetic(_) => Ok(crate::scenario_helpers::final_response(model, "2")),
            Self::BootstrapImportFailure(_) => {
                crate::bootstrap_failure_provider::BootstrapImportFailureProvider::respond(
                    context, model,
                )
            }
            Self::SelfHostingPhp(scenario) => scenario.respond(context, model),
            Self::Echo => Ok(crate::scenario_helpers::final_response(
                model,
                &crate::scenario_helpers::user_prompt(context),
            )),
            #[cfg(test)]
            Self::Faux(_) => Ok(crate::scenario_helpers::error_response(
                model,
                "faux script was exhausted".to_owned(),
            )),
            #[cfg(test)]
            Self::Continuation(_) => Ok(crate::scenario_helpers::error_response(
                model,
                "continuation scenario provider is used directly".to_owned(),
            )),
        };
        result.unwrap_or_else(|message| crate::scenario_helpers::error_response(model, message))
    }
}
