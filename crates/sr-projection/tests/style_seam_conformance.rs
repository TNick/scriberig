//! Proves the domain-owned projection style seam and its compatibility defaults.

use sr_projection::{
    DiagnosticKind, DiagnosticStage, FidelityAssessment, ProjectionFidelity, ProjectionKind,
    ProjectionPurpose, ProjectionStyleAdapter, ResolvedStyle, StyleApplicability,
    StylePropertyDescriptor, StyleRef, StyleSchema, StyleSchemaRef, StyleValue, StyleValueKind,
    invalid_style_diagnostic,
};

/// Builds the generic kind these seam tests use.
///
/// # Returns
/// The validated projection kind.
fn generic_kind() -> ProjectionKind {
    ProjectionKind::new("test.generic").expect("test kind")
}

/// A style adapter that implements none of the seam.
///
/// # Public members
/// - None.
///
/// # Private members
/// - None.
struct BareStyleAdapter;

impl ProjectionStyleAdapter for BareStyleAdapter {}

/// A style adapter whose "domain" is two opaque option bytes.
///
/// It exists only to prove the seam carries adapter-owned meaning: canonical
/// form is one option byte plus one toggle byte, merge keeps the base option and
/// takes the overlay toggle, and fidelity degrades under one exact option value.
/// No language-specific property name appears anywhere.
///
/// # Public members
/// - `encode`: Builds canonical style bytes.
///
/// # Private members
/// - None.
struct TwoOptionStyleAdapter;

impl TwoOptionStyleAdapter {
    /// Builds canonical style bytes.
    ///
    /// - `option`: First adapter option byte.
    /// - `toggle`: Second adapter option byte.
    ///
    /// # Returns
    /// The canonical style value.
    fn encode(option: u8, toggle: u8) -> StyleValue {
        StyleValue::new(vec![option, toggle])
    }
}

impl ProjectionStyleAdapter for TwoOptionStyleAdapter {
    /// Declares one schema with two neutral properties.
    ///
    /// - `self`: Adapter declaring its schema.
    /// - `kind`: Kind being inspected.
    ///
    /// # Returns
    /// The schema for the generic test kind, otherwise `None`.
    fn style_schema(&self, kind: &ProjectionKind) -> Option<StyleSchema> {
        if kind != &generic_kind() {
            return None;
        }
        StyleSchema::new(
            "test.generic",
            "2",
            vec![kind.clone()],
            vec![StylePropertyDescriptor {
                key: "option-alpha".to_owned(),
                label: "Alpha".to_owned(),
                description: None,
                kind: StyleValueKind::Enum,
                allowed_values: vec!["1".to_owned(), "2".to_owned()],
                minimum: None,
                maximum: None,
                default: b"1".to_vec(),
                applicability: StyleApplicability::Both,
                affects_fidelity: false,
                required_for_materialization: false,
            }],
        )
        .ok()
    }

    /// Returns the adapter's exact default style.
    ///
    /// - `self`: Adapter owning the default.
    /// - `kind`: Kind being rendered.
    ///
    /// # Returns
    /// The canonical default style carrying its schema reference.
    ///
    /// # Errors
    /// Returns a structured diagnostic when the default pair is inconsistent.
    fn default_style(
        &self,
        kind: &ProjectionKind,
    ) -> sr_projection::ProjectionResult<ResolvedStyle> {
        let value = Self::encode(b'1', b'1');
        let resolved = ResolvedStyle::new(
            StyleRef::Value(value.clone()),
            value,
            Some(StyleSchemaRef::new("test.generic", "2").expect("schema reference")),
        )
        .map_err(|error| {
            invalid_style_diagnostic(kind, "bad-default", &error.to_string(), Vec::new())
        })?;
        Ok(resolved)
    }

    /// Normalizes any two-byte input to canonical two-byte form.
    ///
    /// - `self`: Adapter owning normalization.
    /// - `kind`: Kind the style renders.
    /// - `input`: Style input to normalize.
    ///
    /// # Returns
    /// Canonical style bytes.
    ///
    /// # Errors
    /// Returns a structured diagnostic when the input is not exactly two bytes.
    fn normalize(
        &self,
        kind: &ProjectionKind,
        input: &StyleValue,
    ) -> sr_projection::ProjectionResult<StyleValue> {
        if input.as_bytes().len() != 2 {
            return Err(Box::new(invalid_style_diagnostic(
                kind,
                "invalid-style-width",
                "style input must carry exactly two adapter option bytes",
                input.as_bytes().to_vec(),
            )));
        }
        Ok(StyleValue::new(input.as_bytes().to_vec()))
    }

    /// Merges two canonical styles by keeping the base option and the overlay
    /// toggle.
    ///
    /// - `self`: Adapter owning merge meaning.
    /// - `_kind`: Kind the style renders, unused by this domain.
    /// - `base`: Lower-precedence canonical style.
    /// - `overlay`: Higher-precedence canonical style.
    ///
    /// # Returns
    /// The domain-merged canonical style.
    ///
    /// # Errors
    /// Returns a diagnostic when either input lacks the canonical width.
    fn merge(
        &self,
        _kind: &ProjectionKind,
        base: &StyleValue,
        overlay: &StyleValue,
    ) -> sr_projection::ProjectionResult<StyleValue> {
        let option = *base.as_bytes().first().ok_or_else(|| {
            Box::new(invalid_style_diagnostic(
                &generic_kind(),
                "invalid-base-width",
                "base style carries no option byte",
                base.as_bytes().to_vec(),
            ))
        })?;
        let toggle = *overlay.as_bytes().get(1).ok_or_else(|| {
            Box::new(invalid_style_diagnostic(
                &generic_kind(),
                "invalid-overlay-width",
                "overlay style carries no toggle byte",
                overlay.as_bytes().to_vec(),
            ))
        })?;
        Ok(Self::encode(option, toggle))
    }

    /// Classifies fidelity from the exact resolved style.
    ///
    /// - `self`: Adapter owning fidelity.
    /// - `_kind`: Kind being rendered, unused by this domain.
    /// - `style`: Exact resolved style the renderer will consume.
    /// - `_purpose`: Why the projection is produced, unused by this domain.
    ///
    /// # Returns
    /// Loss-aware editable when the resolved style hides a category, otherwise
    /// lossless editable.
    fn classify(
        &self,
        _kind: &ProjectionKind,
        style: &ResolvedStyle,
        _purpose: ProjectionPurpose,
    ) -> FidelityAssessment {
        if style.value().as_bytes().first() == Some(&b'2') {
            return FidelityAssessment::loss_aware(
                vec!["option-beta".to_owned()],
                Some("the resolved style omits one canonical category".to_owned()),
            );
        }
        FidelityAssessment::lossless()
    }
}

/// Verifies an unmigrated adapter resolves to one default and conservative fidelity.
#[test]
fn unmigrated_adapter_resolves_to_conservative_defaults() {
    let adapter = BareStyleAdapter;
    let target = generic_kind();

    assert!(adapter.style_schema(&target).is_none());
    let missing_default = adapter
        .default_style(&target)
        .expect_err("no declared default");
    assert_eq!(missing_default.kind, DiagnosticKind::UnsupportedOperation);
    assert_eq!(missing_default.stage, DiagnosticStage::StyleResolution);

    let assessment = adapter.classify(
        &target,
        &ResolvedStyle::inline(StyleValue::new(b"any".to_vec())),
        ProjectionPurpose::InteractiveView,
    );
    assert_eq!(assessment.fidelity, ProjectionFidelity::ReadOnlyLossy);
    assert!(assessment.explanation.is_some());
}

/// Verifies an adapter-owned style seam owns normalization, merge, and fidelity.
#[test]
fn adapter_style_seam_owns_meaning_and_fidelity() {
    let adapter = TwoOptionStyleAdapter;
    let target = generic_kind();
    let schema = adapter.style_schema(&target).expect("declared schema");

    let normalized = adapter
        .normalize(&target, &StyleValue::new(vec![b'1', b'0']))
        .expect("normalizable input");
    assert_eq!(normalized.as_bytes(), b"10");
    let invalid = adapter
        .normalize(&target, &StyleValue::new(vec![b'1']))
        .expect_err("invalid width");
    assert_eq!(invalid.kind, DiagnosticKind::InvalidConfiguration);
    assert_eq!(invalid.stage, DiagnosticStage::StyleResolution);
    assert_eq!(invalid.detail.code, "invalid-style-width");

    let merged = adapter
        .merge(
            &target,
            &TwoOptionStyleAdapter::encode(b'1', b'1'),
            &TwoOptionStyleAdapter::encode(b'2', b'0'),
        )
        .expect("domain merge");
    assert_eq!(merged.as_bytes(), b"10");
    adapter
        .merge(
            &target,
            &StyleValue::new(Vec::new()),
            &StyleValue::new(b"10".to_vec()),
        )
        .expect_err("base must carry an option byte");

    let lossless = adapter.classify(
        &target,
        &ResolvedStyle::inline(TwoOptionStyleAdapter::encode(b'1', b'1')),
        ProjectionPurpose::SharedMaterialization,
    );
    let lossy = adapter.classify(
        &target,
        &ResolvedStyle::inline(TwoOptionStyleAdapter::encode(b'2', b'1')),
        ProjectionPurpose::SharedMaterialization,
    );
    assert_eq!(lossless.fidelity, ProjectionFidelity::LosslessEditable);
    assert_eq!(lossy.fidelity, ProjectionFidelity::LossAwareEditable);
    assert!(lossy.omitted_categories.contains(&"option-beta".to_owned()));
    assert!(
        adapter
            .default_style(&target)
            .expect("default")
            .schema()
            .is_some()
    );
    assert_eq!(schema.properties.len(), 1);
}
