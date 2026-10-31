//! Provides shared projection style fixtures for application integration tests.

use std::collections::BTreeMap;

use sr_projection::{
    FidelityAssessment, ProjectionDiagnostic, ProjectionKind, ProjectionPurpose,
    ProjectionStyleAdapter, ResolvedStyle, StyleApplicability, StylePropertyDescriptor,
    StyleSchema, StyleValue, StyleValueKind, invalid_style_diagnostic,
};

/// Kind label handled by the domain style adapter under test.
pub const KIND: &str = "document";

/// Builds the projection kind under test.
///
/// # Returns
/// The validated document projection kind.
/// # Panics
/// Panics when the fixture kind label is invalid.
#[must_use]
pub fn kind() -> ProjectionKind {
    ProjectionKind::new(KIND).expect("kind label is valid")
}

/// Encodes one style as the domain's canonical field map.
///
/// - `pairs`: Style field pairs to encode.
///
/// # Returns
/// The exact adapter-owned style value.
/// # Panics
/// Panics when the fixture style map cannot be encoded.
#[must_use]
pub fn style(pairs: &[(&str, &str)]) -> StyleValue {
    let mut map = BTreeMap::new();
    for (key, value) in pairs {
        map.insert((*key).to_owned(), (*value).to_owned());
    }
    StyleValue::new(serde_json::to_vec(&map).expect("style json encodes"))
}

/// Decodes one style back into its field map.
///
/// - `value`: Style value produced by the fixture adapter.
///
/// # Returns
/// The decoded style fields.
/// # Panics
/// Panics when a fixture style value cannot be decoded.
#[must_use]
pub fn fields(value: &StyleValue) -> BTreeMap<String, String> {
    serde_json::from_slice(value.as_bytes()).expect("style json decodes")
}

/// Reads one field out of a decoded style.
///
/// - `fields`: Decoded style fields.
/// - `key`: Field name to read.
///
/// # Returns
/// The field value when the style declares it.
pub fn field<'a>(fields: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    fields.get(key).map(String::as_str)
}

/// Builds one enum property descriptor for the fixture schema.
///
/// - `key`: Property name to describe.
/// - `allowed`: Enum members the property accepts.
///
/// # Returns
/// The property descriptor.
pub fn property(key: &str, allowed: &[&str]) -> StylePropertyDescriptor {
    StylePropertyDescriptor {
        key: key.to_owned(),
        label: key.to_owned(),
        description: None,
        kind: StyleValueKind::Enum,
        allowed_values: allowed.iter().map(|value| (*value).to_owned()).collect(),
        minimum: None,
        maximum: None,
        default: allowed
            .first()
            .map(|value| (*value).to_owned())
            .map(String::into_bytes)
            .unwrap_or_default(),
        applicability: StyleApplicability::Both,
        affects_fidelity: true,
        required_for_materialization: false,
    }
}

/// A domain style adapter with per-field override merge and field validation.
///
/// The fixture encodes styles as a JSON field map so the tests can prove that
/// normalization, merging, and fidelity are domain decisions rather than shared
/// layering.
pub struct DocumentStyleAdapter;

impl DocumentStyleAdapter {
    /// Returns whether the domain accepts one style field and value pair.
    ///
    /// - `key`: Field name to check.
    /// - `value`: Field value to check.
    ///
    /// # Returns
    /// Whether the domain defines that field with that value.
    fn accepts(key: &str, value: &str) -> bool {
        match key {
            "indent" => matches!(value, "spaces" | "tabs"),
            "width" => matches!(value, "80" | "100"),
            "emphasis" => matches!(value, "bold" | "italic"),
            _ => false,
        }
    }

    /// Rejects every field pair the domain does not define.
    ///
    /// - `value`: Style value to validate.
    /// - `kind`: Projection kind whose domain owns the field set.
    ///
    /// # Returns
    /// Unit when every field pair is defined by the domain.
    ///
    /// # Errors
    /// Returns a structured invalid-configuration diagnostic naming the first
    /// rejected pair.
    fn validate(
        value: &StyleValue,
        kind: &ProjectionKind,
    ) -> Result<(), Box<ProjectionDiagnostic>> {
        for (key, field_value) in &fields(value) {
            if !Self::accepts(key, field_value) {
                return Err(Box::new(invalid_style_diagnostic(
                    kind,
                    "style-field-rejected",
                    &format!("the document domain rejects style field {key}={field_value}"),
                    value.as_bytes().to_vec(),
                )));
            }
        }
        Ok(())
    }
}

impl ProjectionStyleAdapter for DocumentStyleAdapter {
    /// Declares the fixture style schema for the document kind.
    ///
    /// - `self`: Adapter declaring its schema.
    /// - `_kind`: Projection kind being inspected.
    ///
    /// # Returns
    /// The three-property document style schema.
    fn style_schema(&self, _kind: &ProjectionKind) -> Option<StyleSchema> {
        Some(
            StyleSchema::new(
                "document-style",
                "1",
                vec![kind()],
                vec![
                    property("indent", &["spaces", "tabs"]),
                    property("width", &["80", "100"]),
                    property("emphasis", &["bold", "italic"]),
                ],
            )
            .expect("schema is consistent"),
        )
    }

    /// Supplies the document domain default style.
    ///
    /// - `self`: Adapter owning the default.
    /// - `_kind`: Projection kind being rendered.
    ///
    /// # Returns
    /// The fixed default style value.
    ///
    /// # Errors
    /// Never fails for the fixture adapter.
    fn default_style(
        &self,
        _kind: &ProjectionKind,
    ) -> Result<ResolvedStyle, Box<ProjectionDiagnostic>> {
        Ok(ResolvedStyle::inline(style(&[
            ("emphasis", "bold"),
            ("indent", "spaces"),
            ("width", "80"),
        ])))
    }

    /// Canonicalizes one style input by validating every field pair.
    ///
    /// - `self`: Adapter owning normalization.
    /// - `kind`: Projection kind whose domain defines the fields.
    /// - `input`: Style input to validate.
    ///
    /// # Returns
    /// The validated style value.
    ///
    /// # Errors
    /// Returns a structured diagnostic for an undefined field pair.
    fn normalize(
        &self,
        kind: &ProjectionKind,
        input: &StyleValue,
    ) -> Result<StyleValue, Box<ProjectionDiagnostic>> {
        Self::validate(input, kind)?;
        Ok(input.clone())
    }

    /// Merges one style by overriding individual domain fields.
    ///
    /// - `self`: Adapter owning merge meaning.
    /// - `kind`: Projection kind whose domain defines the fields.
    /// - `base`: Lower-precedence style.
    /// - `overlay`: Higher-precedence style.
    ///
    /// # Returns
    /// The merged style value.
    ///
    /// # Errors
    /// Returns a structured diagnostic when the overlay names an undefined pair.
    fn merge(
        &self,
        kind: &ProjectionKind,
        base: &StyleValue,
        overlay: &StyleValue,
    ) -> Result<StyleValue, Box<ProjectionDiagnostic>> {
        Self::validate(overlay, kind)?;
        let mut merged = fields(base);
        merged.extend(fields(overlay));
        Ok(style(
            &merged
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str()))
                .collect::<Vec<_>>(),
        ))
    }

    /// Classifies fidelity from the exact resolved style.
    ///
    /// - `self`: Adapter owning fidelity.
    /// - `_kind`: Projection kind being rendered.
    /// - `style`: Exact resolved style a renderer would consume.
    /// - `_purpose`: Purpose, unused because the fixture's loss is style-driven.
    ///
    /// # Returns
    /// A lossless, loss-aware, or read-only assessment for that exact style.
    fn classify(
        &self,
        _kind: &ProjectionKind,
        style: &ResolvedStyle,
        _purpose: ProjectionPurpose,
    ) -> FidelityAssessment {
        let resolved = fields(style.value());
        if field(&resolved, "indent") == Some("tabs") {
            return FidelityAssessment::conservative(
                "the target renderer cannot preserve tab indentation exactly",
            );
        }
        if field(&resolved, "width") == Some("100") {
            return FidelityAssessment::loss_aware(
                vec!["line-width".to_owned()],
                Some("line width 100 is reflowed by the shared target".to_owned()),
            );
        }
        FidelityAssessment::lossless()
    }
}
