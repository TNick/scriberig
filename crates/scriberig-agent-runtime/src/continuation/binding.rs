//! Names the exact `ScribeRig` execution one provider continuation belongs to.

use rpi_ai::{Model, ThinkingLevel};

/// Exact execution identity a retained provider handle is bound to.
///
/// A handle may be reused only when every component still names the execution
/// that produced it. `context_fingerprint` is the fingerprint of the canonical
/// compiled provider context for the completed lineage, so a changed branch
/// lineage, compiled profile or component revision, dependency set, provider
/// context, or effective semantic grant counts as divergence.
///
/// # Public members
/// - `provider_id`: Upstream provider identity that produced the handle.
/// - `model`: Exact upstream model row selected for that execution.
/// - `profile`: Host connection/profile identity selected with the model.
/// - `thinking_level`: Upstream thinking level applied to the provider chain.
/// - `context_fingerprint`: Fingerprint of the canonical compiled provider context.
///
/// # Private members
/// None.
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuationBinding {
    /// Upstream provider identity that produced the handle.
    pub provider_id: String,
    /// Exact upstream model row selected for that execution.
    pub model: Model,
    /// Host connection/profile identity selected with the model.
    pub profile: String,
    /// Upstream thinking level applied to the provider chain.
    pub thinking_level: ThinkingLevel,
    /// Fingerprint of the canonical compiled provider context.
    pub context_fingerprint: String,
}

impl ContinuationBinding {
    /// Builds one binding from the exact resolved execution inputs.
    ///
    /// # Parameters
    /// - `provider_id`: Upstream provider identity for the selection.
    /// - `model`: Exact upstream model row for the selection.
    /// - `profile`: Host connection/profile identity for the selection.
    /// - `thinking_level`: Upstream thinking level for the selection.
    /// - `context_fingerprint`: Fingerprint of the canonical compiled context.
    ///
    /// # Returns
    /// The binding a handle is stored under and revalidated against.
    #[must_use]
    pub fn new(
        provider_id: impl Into<String>,
        model: Model,
        profile: impl Into<String>,
        thinking_level: ThinkingLevel,
        context_fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            model,
            profile: profile.into(),
            thinking_level,
            context_fingerprint: context_fingerprint.into(),
        }
    }

    /// Reports whether a handle bound to `previous` is current for this execution.
    ///
    /// # Parameters
    /// - `previous`: Binding a retained handle was produced under.
    ///
    /// # Returns
    /// `true` only when provider, model, connection profile, thinking level, and
    /// compiled-context fingerprint all still match.
    #[must_use]
    pub fn matches(&self, previous: &Self) -> bool {
        self == previous
    }
}
