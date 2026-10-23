//! Describes adapter-owned style schemas for structured client presentation.

use crate::model::ProjectionKind;
use crate::style::StyleSchemaRef;

/// Names the generic value shape of one style property.
///
/// The kinds describe how a client may present and validate the value. They
/// never define what the property means inside a domain.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StyleValueKind {
    /// Two-state flag.
    Boolean,
    /// Bounded or unbounded integer.
    Integer,
    /// Value drawn from an explicit allowed set.
    Enum,
    /// Free text such as a marker sequence or name.
    Text,
    /// Structured adapter bytes a generic client presents read-only.
    Opaque,
}

/// States which projection purposes a style property may legitimately affect.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StyleApplicability {
    /// Applies only to a personal interactive view.
    ViewOnly,
    /// Applies only to shared materialization.
    MaterializationOnly,
    /// Applies to both interactive view and shared materialization.
    Both,
}

/// Describes one adapter-owned style property for generic presentation.
///
/// # Public members
/// - `key`: Stable adapter-owned property identifier.
/// - `label`: Human-readable label safe for clients.
/// - `description`: Optional longer explanation safe for clients.
/// - `kind`: Generic value shape.
/// - `allowed_values`: Enum members when the kind is `Enum`.
/// - `minimum` and `maximum`: Integer bounds when the kind is `Integer`.
/// - `default`: Canonical encoded default value bytes.
/// - `applicability`: Purposes the property may affect.
/// - `affects_fidelity`: Whether changing the value can change fidelity.
/// - `required_for_materialization`: Whether materialization may omit it.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StylePropertyDescriptor {
    /// Stable adapter-owned property identifier.
    pub key: String,
    /// Human-readable label safe for client presentation.
    pub label: String,
    /// Optional longer explanation safe for client presentation.
    pub description: Option<String>,
    /// Generic value shape.
    pub kind: StyleValueKind,
    /// Allowed enum members when `kind` is `Enum`.
    pub allowed_values: Vec<String>,
    /// Inclusive lower bound when `kind` is `Integer`.
    pub minimum: Option<i64>,
    /// Inclusive upper bound when `kind` is `Integer`.
    pub maximum: Option<i64>,
    /// Canonical encoded default value bytes.
    pub default: Vec<u8>,
    /// Projection purposes this property may affect.
    pub applicability: StyleApplicability,
    /// Whether this property can change projection fidelity.
    pub affects_fidelity: bool,
    /// Whether shared materialization must keep this property.
    pub required_for_materialization: bool,
}

/// Reports an inconsistent style schema descriptor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StyleSchemaError {
    /// A property key or label was empty.
    EmptyPropertyKey,
    /// An enum property declared no allowed values.
    EmptyEnumDomain,
    /// An integer range had its bounds reversed.
    ReversedRange,
    /// Two descriptors repeated one property key.
    DuplicatePropertyKey,
    /// A schema identified an empty adapter or version label.
    EmptySchemaIdentifier,
}

/// Groups the adapter-owned style contract for a set of projection kinds.
///
/// A schema is descriptive metadata that lets a generic client build a settings
/// editor. It is never a formatter: validation and interpretation of concrete
/// style values remain with the domain adapter that published the schema.
///
/// # Public members
/// - `reference`: Exact adapter and version identity of this schema.
/// - `projection_kinds`: Kinds this schema interprets.
/// - `properties`: Ordered property descriptors.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StyleSchema {
    /// Exact adapter and version identity of this schema.
    pub reference: StyleSchemaRef,
    /// Projection kinds this schema interprets.
    pub projection_kinds: Vec<ProjectionKind>,
    /// Ordered property descriptors.
    pub properties: Vec<StylePropertyDescriptor>,
}

impl StyleSchema {
    /// Builds a validated schema for one adapter and its kinds.
    ///
    /// - `adapter_id`: Stable adapter or schema identifier.
    /// - `version`: Adapter-owned schema version label.
    /// - `projection_kinds`: Kinds the schema interprets.
    /// - `properties`: Ordered property descriptors.
    ///
    /// # Returns
    /// The validated schema.
    ///
    /// # Errors
    /// Returns a structured error when identifiers are empty or a descriptor is
    /// internally inconsistent.
    pub fn new(
        adapter_id: impl Into<String>,
        version: impl Into<String>,
        projection_kinds: Vec<ProjectionKind>,
        properties: Vec<StylePropertyDescriptor>,
    ) -> Result<Self, StyleSchemaError> {
        let reference = StyleSchemaRef::new(adapter_id, version)
            .map_err(|_| StyleSchemaError::EmptySchemaIdentifier)?;
        let mut seen: Vec<&str> = Vec::with_capacity(properties.len());
        for property in &properties {
            property.validate()?;
            if seen.contains(&property.key.as_str()) {
                return Err(StyleSchemaError::DuplicatePropertyKey);
            }
            seen.push(property.key.as_str());
        }
        Ok(Self {
            reference,
            projection_kinds,
            properties,
        })
    }

    /// Reports whether this schema interprets one projection kind.
    ///
    /// - `kind`: Projection kind to test.
    ///
    /// # Returns
    /// True when the kind is covered by this schema.
    #[must_use]
    pub fn covers(&self, kind: &ProjectionKind) -> bool {
        self.projection_kinds.contains(kind)
    }

    /// Finds one property descriptor by its stable key.
    ///
    /// - `key`: Property key to locate.
    ///
    /// # Returns
    /// The descriptor when this schema declares that property.
    #[must_use]
    pub fn property(&self, key: &str) -> Option<&StylePropertyDescriptor> {
        self.properties.iter().find(|item| item.key == key)
    }
}

impl StylePropertyDescriptor {
    /// Checks internal consistency of one descriptor.
    ///
    /// - `self`: Descriptor to validate.
    ///
    /// Unit when the descriptor is usable by a generic client.
    ///
    /// # Returns
    /// Success when the descriptor is internally consistent.
    ///
    /// # Errors
    /// Returns a structured error for an empty key, an empty enum domain, or a
    /// reversed integer range.
    pub fn validate(&self) -> Result<(), StyleSchemaError> {
        if self.key.trim().is_empty() || self.label.trim().is_empty() {
            return Err(StyleSchemaError::EmptyPropertyKey);
        }
        if self.kind == StyleValueKind::Enum && self.allowed_values.is_empty() {
            return Err(StyleSchemaError::EmptyEnumDomain);
        }
        if let (Some(minimum), Some(maximum)) = (self.minimum, self.maximum)
            && minimum > maximum
        {
            return Err(StyleSchemaError::ReversedRange);
        }
        Ok(())
    }
}

impl std::fmt::Display for StyleSchemaError {
    /// Formats the schema defect as a stable diagnostic label.
    ///
    /// - `self`: Defect to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatting success or the formatter error.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::EmptyPropertyKey => "style property key or label is empty",
            Self::EmptyEnumDomain => "enum style property declares no allowed values",
            Self::ReversedRange => "integer style property range is reversed",
            Self::DuplicatePropertyKey => "style schema repeats a property key",
            Self::EmptySchemaIdentifier => "style schema identifier is empty",
        };
        formatter.write_str(label)
    }
}

impl std::error::Error for StyleSchemaError {}
