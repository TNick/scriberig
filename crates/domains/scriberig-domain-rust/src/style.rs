//! Declares the Rust domain's rendering style and its projection style adapter.

use sr_projection::{
    FidelityAssessment, ProjectionDiagnostic, ProjectionKind, ProjectionPurpose,
    ProjectionStyleAdapter, ResolvedStyle, StyleApplicability, StylePropertyDescriptor,
    StyleSchema, StyleValue, StyleValueKind, invalid_style_diagnostic,
};

/// Projection kind whose source text this domain renders.
pub const RUST_SOURCE_KIND: &str = "rust";

/// Indentation characters the Rust renderer may emit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustIndent {
    /// Indent with spaces at the configured width.
    Spaces,
    /// Indent with one tab character per level.
    Tabs,
}

/// Exact Rust source rendering style owned by the Rust domain.
///
/// The style changes only presentation. Every supported value round-trips
/// through the domain parser, so choosing one never changes semantic identity.
///
/// # Public members
/// - `indent`: Indentation character family.
/// - `indent_width`: Space width per indentation level.
/// - `blank_lines_between_items`: Blank lines between top-level items.
/// - `validate`: Rejects values the renderer does not implement.
/// - `encode`: Produces the canonical style bytes.
/// - `decode`: Reads canonical style bytes.
/// - `indentation`: Returns the indentation unit for one level.
///
/// # Private members
/// - None.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustStyle {
    /// Indentation character family.
    pub indent: RustIndent,
    /// Space width per indentation level.
    pub indent_width: usize,
    /// Blank lines between top-level items.
    pub blank_lines_between_items: usize,
}

impl Default for RustStyle {
    /// Builds the canonical Rust rendering style.
    ///
    /// # Returns
    /// The style the renderer emitted before styles existed.
    fn default() -> Self {
        Self {
            indent: RustIndent::Spaces,
            indent_width: 4,
            blank_lines_between_items: 1,
        }
    }
}

impl RustStyle {
    /// Rejects presentation values the renderer does not implement.
    ///
    /// - `self`: Style to validate.
    ///
    /// # Returns
    /// Unit when every field is one the renderer implements.
    ///
    /// # Errors
    /// Returns the offending field name when a value is unsupported.
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.indent_width, 2 | 4 | 8) {
            return Err("indent-width must be 2, 4, or 8".to_owned());
        }
        if self.blank_lines_between_items > 2 {
            return Err("blank-lines must be 0, 1, or 2".to_owned());
        }
        Ok(())
    }

    /// Returns the indentation unit for one nesting level.
    ///
    /// - `self`: Style supplying the presentation choice.
    ///
    /// # Returns
    /// The exact text inserted for one indentation level.
    #[must_use]
    pub fn indentation(&self) -> String {
        match self.indent {
            RustIndent::Tabs => "\t".to_owned(),
            RustIndent::Spaces => " ".repeat(self.indent_width),
        }
    }

    /// Encodes the style into its canonical domain bytes.
    ///
    /// - `self`: Style to encode.
    ///
    /// # Returns
    /// Canonical field bytes in fixed order.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        format!(
            "indent={},indent-width={},blank-lines={}",
            match self.indent {
                RustIndent::Spaces => "spaces",
                RustIndent::Tabs => "tabs",
            },
            self.indent_width,
            self.blank_lines_between_items
        )
        .into_bytes()
    }

    /// Decodes canonical domain style bytes.
    ///
    /// - `bytes`: Exact style bytes produced by a caller or encoder.
    ///
    /// Each field may appear at most once. Repeated fields are rejected rather
    /// than silently applying a last-write-wins interpretation.
    ///
    /// # Returns
    /// The decoded style when every field is recognized and unique.
    ///
    /// # Errors
    /// Returns the unrecognized or repeated field, or the unsupported value,
    /// naming the field when decoding fails.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let text =
            std::str::from_utf8(bytes).map_err(|_| "style bytes are not utf-8".to_owned())?;
        let mut style = Self::default();
        if text.trim().is_empty() {
            return Ok(style);
        }
        let mut seen: Vec<&str> = Vec::new();
        for entry in text.split(',') {
            let (key, value) = entry
                .split_once('=')
                .ok_or_else(|| format!("style field {entry} has no value"))?;
            let key = key.trim();
            if seen.contains(&key) {
                return Err(format!("duplicate style field {key}"));
            }
            seen.push(key);
            match key {
                "indent" => {
                    style.indent = match value.trim() {
                        "spaces" => RustIndent::Spaces,
                        "tabs" => RustIndent::Tabs,
                        other => return Err(format!("indent must be spaces or tabs, got {other}")),
                    };
                }
                "indent-width" => {
                    style.indent_width = value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| "indent-width must be a number".to_owned())?;
                }
                "blank-lines" => {
                    style.blank_lines_between_items = value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| "blank-lines must be a number".to_owned())?;
                }
                other => return Err(format!("unknown style field {other}")),
            }
        }
        Ok(style)
    }
}

/// Rust domain implementation of the shared projection style seam.
///
/// It declares which style properties exist, canonicalizes and merges style
/// input field by field, supplies the exact default style, and states fidelity
/// for one exact resolved style.
///
/// # Public members
/// - `style_schema`: Declares the three presentation properties.
/// - `default_style`: Supplies the canonical rendering style.
/// - `normalize`: Rejects undefined fields and re-encodes canonically.
/// - `merge`: Overrides individual fields.
/// - `classify`: Reports lossless editability for supported styles.
///
/// # Private members
/// - None.
#[derive(Clone, Copy, Debug, Default)]
pub struct RustStyleAdapter;

impl RustStyleAdapter {
    /// Returns an enum or integer property descriptor for the Rust style schema.
    ///
    /// - `key`: Property name the renderer implements.
    /// - `allowed`: Canonical encoded values accepted for the property.
    ///
    /// # Returns
    /// The property descriptor.
    fn descriptor(key: &str, allowed: &[&str]) -> StylePropertyDescriptor {
        StylePropertyDescriptor {
            key: key.to_owned(),
            label: key.to_owned(),
            description: None,
            kind: if key == "indent" {
                StyleValueKind::Enum
            } else {
                StyleValueKind::Integer
            },
            allowed_values: allowed.iter().map(|value| (*value).to_owned()).collect(),
            minimum: if key == "indent" { None } else { Some(0) },
            maximum: if key == "indent" { None } else { Some(8) },
            default: allowed
                .first()
                .map(|value| (*value).as_bytes().to_vec())
                .unwrap_or_default(),
            applicability: StyleApplicability::Both,
            affects_fidelity: false,
            required_for_materialization: false,
        }
    }
}

impl ProjectionStyleAdapter for RustStyleAdapter {
    /// Declares the Rust domain style schema.
    ///
    /// - `self`: Adapter declaring its schema.
    /// - `kind`: Projection kind being inspected.
    ///
    /// # Returns
    /// The style schema for the Rust source kind, and none for others.
    fn style_schema(&self, kind: &ProjectionKind) -> Option<StyleSchema> {
        if kind.as_str() != RUST_SOURCE_KIND {
            return None;
        }
        StyleSchema::new(
            "rust-source-style",
            "1",
            vec![kind.clone()],
            vec![
                Self::descriptor("indent", &["spaces", "tabs"]),
                Self::descriptor("indent-width", &["4", "2", "8"]),
                Self::descriptor("blank-lines", &["1", "0", "2"]),
            ],
        )
        .ok()
    }

    /// Supplies the exact canonical Rust rendering style.
    ///
    /// - `self`: Adapter owning the default.
    /// - `kind`: Projection kind being rendered.
    ///
    /// # Returns
    /// The default style for the Rust source kind.
    ///
    /// # Errors
    /// Returns an unsupported diagnostic for a kind this domain does not render.
    fn default_style(
        &self,
        kind: &ProjectionKind,
    ) -> Result<ResolvedStyle, Box<ProjectionDiagnostic>> {
        if kind.as_str() != RUST_SOURCE_KIND {
            return Err(Box::new(invalid_style_diagnostic(
                kind,
                "rust-style-kind-unsupported",
                "the Rust domain renders style only for Rust source",
                Vec::new(),
            )));
        }
        Ok(ResolvedStyle::inline(StyleValue::new(
            RustStyle::default().encode(),
        )))
    }

    /// Canonicalizes one style input through the domain's own field set.
    ///
    /// - `self`: Adapter owning normalization.
    /// - `kind`: Projection kind whose domain owns the fields.
    /// - `input`: Style input to validate.
    ///
    /// # Returns
    /// The re-encoded canonical style bytes.
    ///
    /// # Errors
    /// Returns a structured diagnostic naming the rejected field or value.
    fn normalize(
        &self,
        kind: &ProjectionKind,
        input: &StyleValue,
    ) -> Result<StyleValue, Box<ProjectionDiagnostic>> {
        let decoded = RustStyle::decode(input.as_bytes()).map_err(|reason| {
            invalid_style_diagnostic(
                kind,
                "rust-style-field-rejected",
                &reason,
                input.as_bytes().to_vec(),
            )
        })?;
        decoded.validate().map_err(|reason| {
            invalid_style_diagnostic(
                kind,
                "rust-style-value-unsupported",
                &reason,
                input.as_bytes().to_vec(),
            )
        })?;
        Ok(StyleValue::new(decoded.encode()))
    }

    /// Merges two styles by overriding individual domain fields.
    ///
    /// - `self`: Adapter owning merge meaning.
    /// - `kind`: Projection kind whose domain owns the fields.
    /// - `base`: Lower-precedence canonical style.
    /// - `overlay`: Higher-precedence canonical style.
    ///
    /// # Returns
    /// The merged canonical style bytes.
    ///
    /// # Errors
    /// Returns a structured diagnostic when either input is not a Rust style.
    fn merge(
        &self,
        kind: &ProjectionKind,
        base: &StyleValue,
        overlay: &StyleValue,
    ) -> Result<StyleValue, Box<ProjectionDiagnostic>> {
        let mut merged = RustStyle::decode(base.as_bytes()).map_err(|reason| {
            invalid_style_diagnostic(
                kind,
                "rust-style-base-invalid",
                &reason,
                base.as_bytes().to_vec(),
            )
        })?;
        let requested = RustStyle::decode(overlay.as_bytes()).map_err(|reason| {
            invalid_style_diagnostic(
                kind,
                "rust-style-overlay-invalid",
                &reason,
                overlay.as_bytes().to_vec(),
            )
        })?;
        let overlay_text = String::from_utf8_lossy(overlay.as_bytes()).into_owned();
        for entry in overlay_text.split(',') {
            let Some((key, _)) = entry.split_once('=') else {
                continue;
            };
            match key.trim() {
                "indent" => merged.indent = requested.indent,
                "indent-width" => merged.indent_width = requested.indent_width,
                "blank-lines" => {
                    merged.blank_lines_between_items = requested.blank_lines_between_items;
                }
                _ => {}
            }
        }
        merged.validate().map_err(|reason| {
            invalid_style_diagnostic(
                kind,
                "rust-style-value-unsupported",
                &reason,
                overlay.as_bytes().to_vec(),
            )
        })?;
        Ok(StyleValue::new(merged.encode()))
    }

    /// Reports fidelity for one exact resolved Rust style.
    ///
    /// - `self`: Adapter owning fidelity.
    /// - `_kind`: Projection kind being rendered.
    /// - `style`: Exact resolved style a renderer would consume.
    /// - `_purpose`: Purpose, which does not change Rust style fidelity.
    ///
    /// # Returns
    /// A lossless editable assessment for a supported style, or a conservative
    /// read-only assessment when the style cannot be decoded.
    fn classify(
        &self,
        _kind: &ProjectionKind,
        style: &ResolvedStyle,
        _purpose: ProjectionPurpose,
    ) -> FidelityAssessment {
        match RustStyle::decode(style.value().as_bytes()) {
            Ok(decoded) if decoded.validate().is_ok() => FidelityAssessment::lossless(),
            _ => FidelityAssessment::conservative(
                "the resolved style is not one the Rust renderer implements",
            ),
        }
    }
}
