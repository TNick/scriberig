//! Defines retention defaults independently from validity and freshness.

/// Suggests how long registered derivation results should be retained.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum RetentionPolicy {
    /// Results may be discarded and recomputed when storage is needed.
    #[default]
    Disposable,
    /// Retain expensive reusable results when practical.
    ReusableExpensive,
    /// Retain results as historical evidence.
    HistoricalEvidence,
    /// Keep results pinned until explicitly unpinned by an owner.
    Pinned,
}
