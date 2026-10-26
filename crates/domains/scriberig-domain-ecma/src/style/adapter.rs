//! Implements the shared projection style seam for the ECMAScript source kinds.
//!
//! Every answer here is a decision of this domain: which properties exist, what
//! their canonical bytes mean, how two sources combine, and what fidelity a
//! resolved style carries. The shared resolution layer never interprets an
//! ECMAScript field, so the renderer and the schema cannot drift apart.

use sr_projection::{
    FidelityAssessment, ProjectionDiagnostic, ProjectionKind, ProjectionPurpose,
    ProjectionStyleAdapter, ResolvedStyle, StyleApplicability, StylePropertyDescriptor,
    StyleSchema, StyleValue, StyleValueKind, invalid_style_diagnostic,
};

use super::ECMA_SOURCE_KINDS;
use super::model::EcmaStyle;

/// ECMAScript domain implementation of the shared projection style seam.
///
/// It declares which style properties exist for `javascript` and `typescript`,
/// canonicalizes and merges style input field by field, supplies the exact
/// default style, and states fidelity for one exact resolved style. The
/// ECMAScript kind covers two projection kinds because the same renderer and the
/// same three fields serve both language modes.
///
/// # Public members
/// - `style_schema`: Declares the three presentation properties for an ECMAScript kind.
/// - `default_style`: Supplies the canonical rendering style.
/// - `normalize`: Rejects undefined fields and re-encodes canonically.
/// - `merge`: Overrides individual fields.
/// - `classify`: Reports lossless editability for a supported style.
///
/// # Private members
/// - `owns`: Reports whether one projection kind is an ECMAScript source kind.
/// - `descriptor`: Builds one property descriptor for the schema.
#[derive(Clone, Copy, Debug, Default)]
pub struct EcmaStyleAdapter;

impl EcmaStyleAdapter {
    /// Reports whether one projection kind is source text this domain renders.
    ///
    /// - `kind`: Projection kind being styled.
    ///
    /// # Returns
    /// True for `javascript` and `typescript`, and false for every other kind.
    fn owns(kind: &ProjectionKind) -> bool {
        ECMA_SOURCE_KINDS.contains(&kind.as_str())
    }

    /// Returns an enum or integer property descriptor for the style schema.
    ///
    /// - `key`: Property name the renderer implements.
    /// - `allowed`: Canonical encoded values accepted for the property, with the
    ///   default value first so a generic client preselects the domain default.
    ///
    /// # Returns
    /// The property descriptor.
    fn descriptor(key: &str, allowed: &[&str]) -> StylePropertyDescriptor {
        let numeric = key == "indent-width";
        StylePropertyDescriptor {
            key: key.to_owned(),
            label: key.to_owned(),
            description: None,
            kind: if numeric {
                StyleValueKind::Integer
            } else {
                StyleValueKind::Enum
            },
            allowed_values: allowed.iter().map(|value| (*value).to_owned()).collect(),
            minimum: if numeric { Some(0) } else { None },
            maximum: if numeric { Some(8) } else { None },
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

impl ProjectionStyleAdapter for EcmaStyleAdapter {
    /// Declares the ECMAScript domain style schema.
    ///
    /// - `self`: Adapter declaring its schema.
    /// - `kind`: Projection kind being inspected.
    ///
    /// # Returns
    /// The style schema for an ECMAScript source kind, and none for any other
    /// kind, so a host cannot read ECMAScript properties for a foreign kind.
    fn style_schema(&self, kind: &ProjectionKind) -> Option<StyleSchema> {
        if !Self::owns(kind) {
            return None;
        }
        StyleSchema::new(
            "ecma-source-style",
            "1",
            vec![kind.clone()],
            vec![
                Self::descriptor("indent", &["spaces", "tabs"]),
                Self::descriptor("indent-width", &["2", "4", "8"]),
                Self::descriptor("quote", &["double", "single"]),
            ],
        )
        .ok()
    }

    /// Supplies the exact canonical ECMAScript rendering style.
    ///
    /// - `self`: Adapter owning the default.
    /// - `kind`: Projection kind being rendered.
    ///
    /// # Returns
    /// The default style for an ECMAScript source kind.
    ///
    /// # Errors
    /// Returns an unsupported diagnostic for a kind this domain does not render.
    fn default_style(
        &self,
        kind: &ProjectionKind,
    ) -> Result<ResolvedStyle, Box<ProjectionDiagnostic>> {
        if !Self::owns(kind) {
            return Err(Box::new(invalid_style_diagnostic(
                kind,
                "ecma-style-kind-unsupported",
                "the ECMAScript domain renders style only for JavaScript and TypeScript source",
                Vec::new(),
            )));
        }
        Ok(ResolvedStyle::inline(StyleValue::new(
            EcmaStyle::default().encode(),
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
        let decoded = EcmaStyle::decode(input.as_bytes()).map_err(|reason| {
            invalid_style_diagnostic(
                kind,
                "ecma-style-field-rejected",
                &reason,
                input.as_bytes().to_vec(),
            )
        })?;
        decoded.validate().map_err(|reason| {
            invalid_style_diagnostic(
                kind,
                "ecma-style-value-unsupported",
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
    /// Returns a structured diagnostic when either input is not an ECMAScript
    /// style, or when the merged result is a value the renderer does not
    /// implement.
    fn merge(
        &self,
        kind: &ProjectionKind,
        base: &StyleValue,
        overlay: &StyleValue,
    ) -> Result<StyleValue, Box<ProjectionDiagnostic>> {
        let mut merged = EcmaStyle::decode(base.as_bytes()).map_err(|reason| {
            invalid_style_diagnostic(
                kind,
                "ecma-style-base-invalid",
                &reason,
                base.as_bytes().to_vec(),
            )
        })?;
        let requested = EcmaStyle::decode(overlay.as_bytes()).map_err(|reason| {
            invalid_style_diagnostic(
                kind,
                "ecma-style-overlay-invalid",
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
                "quote" => merged.quote = requested.quote,
                _ => {}
            }
        }
        merged.validate().map_err(|reason| {
            invalid_style_diagnostic(
                kind,
                "ecma-style-value-unsupported",
                &reason,
                overlay.as_bytes().to_vec(),
            )
        })?;
        Ok(StyleValue::new(merged.encode()))
    }

    /// Reports fidelity for one exact resolved ECMAScript style.
    ///
    /// - `self`: Adapter owning fidelity.
    /// - `_kind`: Projection kind being rendered.
    /// - `style`: Exact resolved style a renderer would consume.
    /// - `_purpose`: Purpose, which does not change ECMAScript style fidelity.
    ///
    /// Every supported field changes only whitespace or quote characters, and
    /// each rendering re-parses to the identical semantic model, so a supported
    /// style hides no canonical content.
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
        match EcmaStyle::decode(style.value().as_bytes()) {
            Ok(decoded) if decoded.validate().is_ok() => FidelityAssessment::lossless(),
            _ => FidelityAssessment::conservative(
                "the resolved style is not one the ECMAScript renderer implements",
            ),
        }
    }
}
