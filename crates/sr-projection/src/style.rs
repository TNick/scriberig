//! Defines exact universal references to and resolutions of projection style inputs.

use std::fmt;

use scriberig_core::{EntityId, RevisionId};

/// Identifies an immutable style revision or exact adapter-supplied value.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum StyleRef {
    /// References a style entity at one immutable revision.
    Revision {
        /// Stable style entity identity.
        entity: EntityId,
        /// Exact immutable style revision.
        revision: RevisionId,
    },
    /// Supplies exact serialized style bytes owned by an adapter.
    Value(StyleValue),
}

/// Carries an exact adapter-defined style value.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StyleValue(Vec<u8>);

/// Identifies the adapter-owned style schema that interprets resolved values.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StyleSchemaRef {
    /// Stable adapter or schema identifier.
    adapter_id: String,
    /// Adapter-owned schema/version label.
    version: String,
}

/// Carries the exact style value a renderer must apply.
///
/// A resolved style retains the provenance reference that selected it, the
/// canonical adapter-defined value consumed by the renderer, and the optional
/// schema needed to validate that value. Renderers therefore never need to
/// consult storage or ambient user settings to interpret a style revision.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ResolvedStyle {
    /// Exact provenance reference selected by style resolution.
    reference: StyleRef,
    /// Canonical adapter-defined value consumed by the renderer.
    value: StyleValue,
    /// Optional adapter schema that interprets the value.
    schema: Option<StyleSchemaRef>,
}

/// Reports an inconsistent resolved style reference/value pair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StyleResolutionError {
    /// An inline style reference names bytes different from the resolved value.
    ReferenceValueMismatch,
}

// Keep style values immutable after construction.
impl StyleValue {
    /// Creates an exact style value from its canonical bytes.
    ///
    /// - `bytes`: Canonical adapter-defined style representation.
    ///
    /// The exact style value.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Borrows the exact style representation.
    ///
    /// - `self`: Value to inspect.
    ///
    /// The canonical style bytes.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl StyleSchemaRef {
    /// Creates an exact adapter-owned style schema reference.
    ///
    /// - `adapter_id`: Stable adapter or schema identifier.
    /// - `version`: Adapter-owned schema/version label.
    ///
    /// The schema reference when both identifiers are nonempty.
    ///
    /// # Returns
    /// The schema reference.
    ///
    /// # Errors
    /// Returns an error when either identifier is empty or whitespace-only.
    pub fn new(
        adapter_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Result<Self, &'static str> {
        let adapter_id = adapter_id.into();
        let version = version.into();
        if adapter_id.trim().is_empty() || version.trim().is_empty() {
            return Err("style schema identifiers must not be empty");
        }
        Ok(Self {
            adapter_id,
            version,
        })
    }

    /// Returns the stable adapter or schema identifier.
    ///
    /// # Returns
    /// The adapter-owned schema identifier.
    #[must_use]
    pub fn adapter_id(&self) -> &str {
        &self.adapter_id
    }

    /// Returns the adapter-owned schema/version label.
    ///
    /// # Returns
    /// The schema version label.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

impl ResolvedStyle {
    /// Resolves an inline style value without a schema reference.
    ///
    /// - `value`: Canonical adapter-defined inline style bytes.
    ///
    /// # Returns
    /// A resolved style whose reference names the same exact bytes.
    #[must_use]
    pub fn inline(value: StyleValue) -> Self {
        Self {
            reference: StyleRef::Value(value.clone()),
            value,
            schema: None,
        }
    }

    /// Binds an exact provenance reference to the value a renderer consumes.
    ///
    /// - `reference`: Style identity selected by resolution.
    /// - `value`: Canonical adapter-defined style bytes.
    /// - `schema`: Optional adapter schema interpreting the value.
    ///
    /// # Returns
    /// The validated resolved style.
    ///
    /// # Errors
    /// Returns an error when an inline reference names different bytes than
    /// the resolved value.
    pub fn new(
        reference: StyleRef,
        value: StyleValue,
        schema: Option<StyleSchemaRef>,
    ) -> Result<Self, StyleResolutionError> {
        if let StyleRef::Value(inline) = &reference
            && *inline != value
        {
            return Err(StyleResolutionError::ReferenceValueMismatch);
        }
        Ok(Self {
            reference,
            value,
            schema,
        })
    }

    /// Returns the exact provenance reference selected by resolution.
    ///
    /// # Returns
    /// The immutable style reference.
    #[must_use]
    pub fn reference(&self) -> &StyleRef {
        &self.reference
    }

    /// Returns the canonical adapter-defined value a renderer consumes.
    ///
    /// # Returns
    /// The exact resolved style bytes.
    #[must_use]
    pub fn value(&self) -> &StyleValue {
        &self.value
    }

    /// Returns the optional adapter schema interpreting this value.
    ///
    /// # Returns
    /// The schema reference when resolution preserved one.
    #[must_use]
    pub fn schema(&self) -> Option<&StyleSchemaRef> {
        self.schema.as_ref()
    }
}

impl fmt::Display for StyleResolutionError {
    /// Formats the resolution failure as a stable diagnostic.
    ///
    /// - `self`: Failure to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatting success or the formatter error.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReferenceValueMismatch => {
                formatter.write_str("inline style reference does not match the resolved value")
            }
        }
    }
}

impl std::error::Error for StyleResolutionError {}

#[cfg(test)]
mod tests {
    use super::{ResolvedStyle, StyleRef, StyleResolutionError, StyleSchemaRef, StyleValue};

    /// Verifies a revision-backed resolution retains exact provenance and value.
    #[test]
    fn revision_resolution_retains_reference_and_value() {
        let entity = scriberig_core::EntityId::from_bytes([1; 16]);
        let revision = scriberig_core::RevisionId::new([2; 32]);
        let style = ResolvedStyle::new(
            StyleRef::Revision { entity, revision },
            StyleValue::new(b"resolved".to_vec()),
            Some(StyleSchemaRef::new("markdown", "1").expect("schema")),
        )
        .expect("resolved revision style");

        assert_eq!(style.reference(), &StyleRef::Revision { entity, revision });
        assert_eq!(style.value().as_bytes(), b"resolved");
        assert_eq!(
            style.schema().map(StyleSchemaRef::adapter_id),
            Some("markdown")
        );
    }

    /// Verifies inline references that disagree with the value are rejected.
    #[test]
    fn inconsistent_inline_reference_is_rejected() {
        let error = ResolvedStyle::new(
            StyleRef::Value(StyleValue::new(b"declared".to_vec())),
            StyleValue::new(b"actual".to_vec()),
            None,
        )
        .expect_err("mismatched inline reference");

        assert_eq!(error, StyleResolutionError::ReferenceValueMismatch);
    }

    /// Verifies an inline resolution preserves the exact reference and value.
    #[test]
    fn inline_resolution_preserves_exact_bytes() {
        let style = ResolvedStyle::inline(StyleValue::new(vec![7, 8, 9]));

        assert_eq!(
            style.reference(),
            &StyleRef::Value(StyleValue::new(vec![7, 8, 9]))
        );
        assert_eq!(style.value().as_bytes(), &[7, 8, 9]);
        assert_eq!(style.schema(), None);
    }
}
