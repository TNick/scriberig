//! Defines property value shapes and property declarations.

use scriberig_core::{KnowledgeClass, KnowledgeValue};

/// Describes the accepted shape of a knowledge value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueShape {
    /// Any value.
    Any,
    /// Null value.
    Null,
    /// Boolean value.
    Bool,
    /// Signed integer.
    SignedInteger,
    /// Unsigned integer.
    UnsignedInteger,
    /// Floating-point value.
    Float,
    /// UTF-8 string.
    String,
    /// Raw bytes.
    Bytes,
    /// List value.
    List,
    /// Record value.
    Record,
}

/// Declares one semantic property.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertySchema {
    /// Stable property key.
    pub key: String,
    /// Human-readable property name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Accepted value shape.
    pub value_shape: ValueShape,
    /// Whether the property must be present.
    pub required: bool,
    /// Canonical or derived ownership.
    pub knowledge_class: KnowledgeClass,
}

/// Reports whether a value matches a declared shape.
///
/// - `value`: Value to inspect.
/// - `shape`: Declared shape.
///
/// `true` when the value matches.
#[must_use]
/// # Returns
/// The operation result.
pub fn matches_shape(value: &KnowledgeValue, shape: &ValueShape) -> bool {
    matches!(
        (value, shape),
        (_, ValueShape::Any)
            | (KnowledgeValue::Null, ValueShape::Null)
            | (KnowledgeValue::Bool(_), ValueShape::Bool)
            | (KnowledgeValue::SignedInteger(_), ValueShape::SignedInteger)
            | (
                KnowledgeValue::UnsignedInteger(_),
                ValueShape::UnsignedInteger
            )
            | (KnowledgeValue::Float(_), ValueShape::Float)
            | (KnowledgeValue::String(_), ValueShape::String)
            | (KnowledgeValue::Bytes(_), ValueShape::Bytes)
            | (KnowledgeValue::List(_), ValueShape::List)
            | (KnowledgeValue::Record(_), ValueShape::Record)
    )
}
