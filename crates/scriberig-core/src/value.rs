//! Defines domain-neutral property values used by semantic knowledge records.

use std::collections::BTreeMap;

/// Represents local property payloads without embedding domain semantics.
///
/// Semantic relationships belong in typed graph relations and collections rather
/// than arbitrary value variants. This type is a payload vocabulary only.
#[derive(Clone, Debug, PartialEq)]
pub enum KnowledgeValue {
    /// An explicitly absent value.
    Null,
    /// A boolean value.
    Bool(bool),
    /// A signed integer value.
    SignedInteger(i64),
    /// An unsigned integer value.
    UnsignedInteger(u64),
    /// A floating-point value.
    Float(f64),
    /// A UTF-8 string value.
    String(String),
    /// An opaque byte payload.
    Bytes(Vec<u8>),
    /// An ordered sequence of values.
    List(Vec<Self>),
    /// A deterministically ordered string-keyed record.
    Record(BTreeMap<String, Self>),
}

/// Classifies whether a value participates in canonical or derived state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KnowledgeClass {
    /// Accepted semantic state.
    Canonical,
    /// Computed or interpreted state that does not itself advance snapshots.
    Derived,
}
