//! Defines freshness policies, immutable observations, and explicit evaluation.

/// Describes how a registered derivation's results age over time.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum FreshnessPolicy {
    /// Exact local inputs remain suitable regardless of elapsed time.
    #[default]
    Immutable,
    /// Results require source validation before asserting currentness.
    RequiresRevalidation,
    /// Results use an adapter-defined freshness policy identifier.
    SourcePolicy(String),
}

/// Captures one historical observation from an external source.
///
/// Observation metadata never changes the recorded fact; expiry only changes
/// whether it satisfies a request for current information.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FreshnessObservation {
    /// Stable external source identifier.
    pub source: String,
    /// Caller-supplied deterministic timestamp units.
    pub observed_at: u64,
    /// Exclusive freshness deadline, when known.
    pub valid_until: Option<u64>,
    /// Optional source revision, `ETag`, or validator token.
    pub validator: Option<String>,
}

/// Reports whether an observation satisfies a freshness request.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FreshnessState {
    /// Observation is suitable at the evaluation time.
    Fresh,
    /// Observation has passed its known freshness deadline.
    Stale,
    /// Source must be checked before currentness can be asserted.
    RequiresRevalidation,
    /// Freshness does not apply to this local immutable result.
    NotApplicable,
}

/// Evaluates a policy against observation metadata at an explicit time.
///
/// - `policy`: Freshness policy selected for the request.
/// - `observation`: Optional exact source observation.
/// - `now`: Deterministic caller-supplied timestamp.
///
/// Structured freshness state. No record or canonical snapshot is modified.
#[must_use]
/// # Returns
/// The operation result.
pub fn evaluate_freshness(
    policy: &FreshnessPolicy,
    observation: Option<&FreshnessObservation>,
    now: u64,
) -> FreshnessState {
    match policy {
        FreshnessPolicy::Immutable => FreshnessState::NotApplicable,
        FreshnessPolicy::RequiresRevalidation => FreshnessState::RequiresRevalidation,
        FreshnessPolicy::SourcePolicy(_) => match observation {
            Some(value) if value.valid_until.is_some_and(|deadline| now >= deadline) => {
                FreshnessState::Stale
            }
            Some(_) => FreshnessState::Fresh,
            None => FreshnessState::RequiresRevalidation,
        },
    }
}
