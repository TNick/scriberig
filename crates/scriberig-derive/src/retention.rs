//! Defines retention defaults independently from validity and freshness.
//!
//! Retention answers how strongly an artifact should be kept under storage
//! pressure. It does not establish whether the artifact satisfies a derivation
//! request. Weak cache references do not pin canonical history; pinned
//! historical evidence can request strong preservation of exact inputs.

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

/// Describes whether a derived result preserves its canonical inputs strongly.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum RetentionStrength {
    /// Cache references do not pin canonical input history.
    #[default]
    WeakDependency,
    /// Historical evidence requests preservation of exact canonical inputs.
    StrongPinnedDependency,
}
