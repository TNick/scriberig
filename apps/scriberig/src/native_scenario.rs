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
    /// Scripted upstream provider supplied by a focused execution test.
    #[cfg(test)]
    Provider(std::sync::Arc<dyn rpi_ai::Provider>),
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

        #[cfg(test)]
        if let Self::Provider(provider) = self {
            return provider;
        }

        let script = FauxScript::new();
        script.set_responses(vec![scenario_response_step(Arc::new(self), model.clone())]);
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
            #[cfg(test)]
            Self::Provider(_) => Ok(crate::scenario_helpers::error_response(
                model,
                "scripted provider is used directly".to_owned(),
            )),
        };
        result.unwrap_or_else(|message| crate::scenario_helpers::error_response(model, message))
    }
}

/// Creates a persistent context-driven faux response for every scenario request.
///
/// The upstream repeating factory remains queued while several provider calls
/// are active, avoiding a queue-empty race between concurrent requests.
///
/// # Parameters
/// - `scenario`: Scenario used to produce the next assistant message.
/// - `model`: Exact model descriptor attached to the response.
///
/// # Returns
/// An upstream repeating factory step.
fn scenario_response_step(scenario: Arc<NativeScenario>, model: Model) -> FauxStep {
    FauxStep::repeating_factory(move |context| scenario.respond(context, &model))
}

#[cfg(test)]
mod tests {
    use super::{NativeScenario, scenario_response_step};
    use rpi_ai::Model;
    use rpi_ai::providers::faux::{FauxProvider, FauxScript, FauxStep};
    use std::sync::Arc;

    /// Selects the upstream repeating factory for a deterministic scenario.
    #[test]
    fn scenario_response_factory_remains_available_without_queue_replenishment() {
        let model: Model = FauxProvider::new(FauxScript::new()).default_model().clone();
        let step = scenario_response_step(Arc::new(NativeScenario::Echo), model);
        let FauxStep::RepeatingFactory(_) = step else {
            panic!("scenario response must be an upstream repeating factory");
        };
    }
}
