//! Defines domain-neutral property values used by semantic knowledge records.

use std::collections::BTreeMap;

use crate::encoding::{CanonicalEncode, CanonicalWriter};

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

impl CanonicalEncode for KnowledgeValue {
    /// Encodes a property value with explicit variant and length markers.
    ///
    /// - `self`: Property value to encode.
    /// - `writer`: Canonical byte output.
    fn encode_canonical(&self, writer: &mut CanonicalWriter) {
        match self {
            Self::Null => writer.write_u8(0),
            Self::Bool(value) => {
                writer.write_u8(1);
                writer.write_bool(*value);
            }
            Self::SignedInteger(value) => {
                writer.write_u8(2);
                writer.write_u64(u64::from_ne_bytes(value.to_ne_bytes()));
            }
            Self::UnsignedInteger(value) => {
                writer.write_u8(3);
                writer.write_u64(*value);
            }
            Self::Float(value) => {
                writer.write_u8(4);
                let bits = if value.is_nan() {
                    f64::NAN.to_bits()
                } else {
                    value.to_bits()
                };
                writer.write_u64(bits);
            }
            Self::String(value) => {
                writer.write_u8(5);
                writer.write_str(value);
            }
            Self::Bytes(value) => {
                writer.write_u8(6);
                writer.write_bytes(value);
            }
            Self::List(values) => {
                writer.write_u8(7);
                writer.write_sequence_len(values.len());
                for value in values {
                    value.encode_canonical(writer);
                }
            }
            Self::Record(values) => {
                writer.write_u8(8);
                writer.write_sequence_len(values.len());
                for (key, value) in values {
                    writer.write_str(key);
                    value.encode_canonical(writer);
                }
            }
        }
    }
}

/// Classifies whether a value participates in canonical or derived state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KnowledgeClass {
    /// Accepted semantic state.
    Canonical,
    /// Computed or interpreted state that does not itself advance snapshots.
    Derived,
}
