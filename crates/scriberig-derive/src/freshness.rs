//! Defines domain-neutral freshness defaults for registered derivations.

/// Describes how a registered derivation's results age over time.
///
/// Freshness changes suitability for a request, not historical identity.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum FreshnessPolicy {
    /// Exact local inputs remain suitable regardless of elapsed time.
    #[default]
    Immutable,
    /// Results rely on source revalidation before asserting currentness.
    RequiresRevalidation,
    /// Results are suitable only through an opaque policy identifier.
    SourcePolicy(String),
}
