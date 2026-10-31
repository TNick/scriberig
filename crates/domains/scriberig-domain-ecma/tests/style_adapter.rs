//! Proves the ECMAScript domain owns projection style meaning, and that one
//! semantic module renders under two exact styles without changing identity.

use std::path::Path;

use scriberig_domain_ecma::source::EcmaSource;
use scriberig_domain_ecma::{
    ECMA_SOURCE_KINDS, EcmaIndent, EcmaQuote, EcmaStyle, EcmaStyleAdapter, parse_source, render,
    render_with_style,
};
use sr_projection::{
    ProjectionFidelity, ProjectionKind, ProjectionPurpose, ProjectionStyleAdapter, ResolvedStyle,
    StyleValue,
};

/// Module used by every style case.
const SAMPLE: &str = "\
import ready from \"state\";
export const threshold = 1.50;
export function gather(value, label) {
  const record = label;
  log(record, threshold);
  if (value) {
    total = value;
  } else {
    total = \"none\";
  }
  return record;
}
";

/// Builds one projection kind.
///
/// - `label`: Kind label to build.
///
/// # Returns
/// The validated projection kind.
///
/// # Panics
/// Panics when the test kind label is invalid.
fn kind(label: &str) -> ProjectionKind {
    ProjectionKind::new(label).expect("kind label is valid")
}

/// Encodes one style in the domain's canonical form.
///
/// - `style`: Style to encode.
///
/// # Returns
/// The exact adapter-owned style value.
fn value(style: EcmaStyle) -> StyleValue {
    StyleValue::new(style.encode())
}

/// Encodes one raw style field list.
///
/// - `text`: Canonical field text one style source would contribute.
///
/// # Returns
/// The style value carrying those bytes.
fn raw(text: &str) -> StyleValue {
    StyleValue::new(text.as_bytes().to_vec())
}

/// Parses the sample module with a fixed path.
///
/// # Returns
/// The semantic model of the supported subset.
fn sample_model() -> EcmaSource {
    parse_source(Path::new("sample.js"), SAMPLE).expect("sample is in the supported subset")
}

/// Decodes one resolved style into the renderer's own style type.
///
/// - `resolved`: Exact resolved style a renderer would consume.
///
/// # Returns
/// The style the renderer must apply.
///
/// # Panics
/// Panics when a test style value is not canonical.
fn decoded(resolved: &ResolvedStyle) -> EcmaStyle {
    EcmaStyle::decode(resolved.value().as_bytes()).expect("style decodes")
}

/// Verifies the schema is declared only for the kinds this domain renders.
#[test]
fn the_schema_is_declared_only_for_the_ecma_source_kinds() {
    for label in ECMA_SOURCE_KINDS {
        let schema = EcmaStyleAdapter
            .style_schema(&kind(label))
            .expect("the domain declares a style schema");
        assert_eq!(
            schema
                .properties
                .iter()
                .map(|property| property.key.as_str())
                .collect::<Vec<_>>(),
            vec!["indent", "indent-width", "quote"],
            "the {label} schema names exactly the fields EcmaStyle encodes"
        );
        assert!(schema.covers(&kind(label)));
    }

    for foreign in ["rust", "markdown", "python"] {
        assert!(
            EcmaStyleAdapter.style_schema(&kind(foreign)).is_none(),
            "the ECMAScript domain declares no schema for {foreign}"
        );
    }
}

/// Verifies the declared default is the domain default and renders identically.
#[test]
fn the_declared_default_style_is_the_domain_default() {
    let default = EcmaStyleAdapter
        .default_style(&kind("javascript"))
        .expect("the domain declares a default style");
    let style = decoded(&default);

    assert_eq!(style, EcmaStyle::default());
    assert_eq!(
        render_with_style(&sample_model(), &style).expect("default style renders"),
        render(&sample_model()).expect("canonical render"),
        "the declared default is the rendering that existed before styles"
    );

    let fidelity = EcmaStyleAdapter.classify(
        &kind("typescript"),
        &default,
        ProjectionPurpose::SharedMaterialization,
    );
    assert_eq!(fidelity.fidelity, ProjectionFidelity::LosslessEditable);
    assert_eq!(
        String::from_utf8(default.value().as_bytes().to_vec()).expect("default bytes are utf-8"),
        "indent=spaces,indent-width=2,quote=double"
    );
}

/// Verifies a foreign kind is refused instead of receiving an ECMAScript style.
#[test]
fn a_foreign_kind_is_refused_the_default_style() {
    let diagnostic = EcmaStyleAdapter
        .default_style(&kind("markdown"))
        .expect_err("the ECMAScript domain does not render markdown");

    assert_eq!(diagnostic.detail.code, "ecma-style-kind-unsupported");
    assert_eq!(
        diagnostic.kind,
        sr_projection::DiagnosticKind::InvalidConfiguration
    );
}

/// Verifies an unknown field is rejected with a structured diagnostic.
#[test]
fn the_adapter_rejects_an_unknown_style_field() {
    let diagnostic = EcmaStyleAdapter
        .normalize(&kind("javascript"), &raw("semicolons=always"))
        .expect_err("the renderer does not implement semicolon control");

    assert_eq!(diagnostic.detail.code, "ecma-style-field-rejected");
    assert!(
        diagnostic
            .message
            .clone()
            .unwrap_or_default()
            .contains("semicolons"),
        "diagnostic: {:?}",
        diagnostic.message
    );
}

/// Verifies the renderer and the adapter refuse an unsupported style value.
#[test]
fn the_renderer_and_adapter_refuse_an_unsupported_style_value() {
    let unsupported = EcmaStyle {
        indent: EcmaIndent::Spaces,
        indent_width: 3,
        quote: EcmaQuote::Double,
    };

    assert!(render_with_style(&sample_model(), &unsupported).is_err());
    let diagnostic = EcmaStyleAdapter
        .normalize(&kind("typescript"), &value(unsupported))
        .expect_err("the domain rejects a three-space indent");

    assert_eq!(diagnostic.detail.code, "ecma-style-value-unsupported");
    assert!(
        diagnostic
            .message
            .clone()
            .unwrap_or_default()
            .contains("indent-width"),
        "diagnostic: {:?}",
        diagnostic.message
    );
}

/// Verifies normalization re-encodes accepted fields in canonical order.
#[test]
fn normalization_re_encodes_accepted_fields_canonically() {
    let normalized = EcmaStyleAdapter
        .normalize(&kind("javascript"), &raw("quote=single,indent-width=4"))
        .expect("accepted fields normalize");

    assert_eq!(
        String::from_utf8(normalized.as_bytes().to_vec()).expect("canonical bytes are utf-8"),
        "indent=spaces,indent-width=4,quote=single"
    );
    assert_eq!(
        normalized,
        EcmaStyleAdapter
            .normalize(&kind("javascript"), &normalized)
            .expect("normalization is idempotent"),
        "a canonical style normalizes to itself"
    );
}

/// Verifies a higher source overrides only the fields it names.
#[test]
fn the_adapter_merges_individual_style_fields() {
    let quoted = EcmaStyleAdapter
        .merge(
            &kind("javascript"),
            &value(EcmaStyle::default()),
            &raw("quote=single"),
        )
        .expect("the domain merges a single field");
    assert_eq!(
        decoded(&ResolvedStyle::inline(quoted)),
        EcmaStyle {
            indent: EcmaIndent::Spaces,
            indent_width: 2,
            quote: EcmaQuote::Single,
        }
    );

    let indented = EcmaStyleAdapter
        .merge(
            &kind("typescript"),
            &value(EcmaStyle::default()),
            &raw("indent=tabs,indent-width=8"),
        )
        .expect("the domain merges two named fields");
    assert_eq!(
        decoded(&ResolvedStyle::inline(indented)),
        EcmaStyle {
            indent: EcmaIndent::Tabs,
            indent_width: 8,
            quote: EcmaQuote::Double,
        }
    );
}

/// Verifies a merge whose combined fields are unsupported is refused.
#[test]
fn the_adapter_refuses_a_merge_the_renderer_cannot_honour() {
    let diagnostic = EcmaStyleAdapter
        .merge(
            &kind("javascript"),
            &raw("indent-width=8"),
            &raw("indent-width=3"),
        )
        .expect_err("a three-space indent is not implemented");

    assert_eq!(diagnostic.detail.code, "ecma-style-value-unsupported");
    let unreadable = EcmaStyleAdapter
        .merge(
            &kind("javascript"),
            &raw("braces=1tbs"),
            &raw("quote=single"),
        )
        .expect_err("the base style is not an ECMAScript style");
    assert_eq!(unreadable.detail.code, "ecma-style-base-invalid");
}

/// Verifies a style the renderer cannot read is classified read-only.
#[test]
fn a_style_the_renderer_cannot_read_is_read_only() {
    let fidelity = EcmaStyleAdapter.classify(
        &kind("javascript"),
        &ResolvedStyle::inline(raw("indent=elastic")),
        ProjectionPurpose::InteractiveView,
    );

    assert_eq!(fidelity.fidelity, ProjectionFidelity::ReadOnlyLossy);
    assert!(!fidelity.fidelity.is_editable());
}

/// Verifies a proposed comment-hiding style is read-only until trivia is modeled.
#[test]
fn comment_hiding_style_is_read_only_until_trivia_has_semantic_identity() {
    let fidelity = EcmaStyleAdapter.classify(
        &kind("javascript"),
        &ResolvedStyle::inline(raw("comments=omit")),
        ProjectionPurpose::InteractiveView,
    );

    assert_eq!(fidelity.fidelity, ProjectionFidelity::ReadOnlyLossy);
    assert!(!fidelity.fidelity.is_editable());
}

/// Verifies two exact styles render one semantic module as different bytes.
#[test]
fn two_exact_styles_render_the_same_semantic_module_differently() {
    let model = sample_model();
    let narrow = value(EcmaStyle::default());
    let quoted = EcmaStyleAdapter
        .merge(&kind("javascript"), &narrow, &raw("quote=single"))
        .expect("personal quoting merges");
    let wide = EcmaStyleAdapter
        .merge(&kind("javascript"), &narrow, &raw("indent-width=4"))
        .expect("workspace width merges");

    let narrow_text = render_with_style(&model, &decoded(&ResolvedStyle::inline(narrow)))
        .expect("canonical render");
    let quoted_text = render_with_style(&model, &decoded(&ResolvedStyle::inline(quoted)))
        .expect("single-quote render");
    let wide_text = render_with_style(&model, &decoded(&ResolvedStyle::inline(wide)))
        .expect("four-space render");

    assert_ne!(narrow_text, quoted_text);
    assert_ne!(narrow_text, wide_text);
    assert_ne!(quoted_text, wide_text);
    assert!(
        narrow_text.contains("from \"state\"") && narrow_text.contains("total = \"none\""),
        "canonical quoting: {narrow_text}"
    );
    assert!(
        quoted_text.contains("from 'state'") && quoted_text.contains("total = 'none'"),
        "single-quote render: {quoted_text}"
    );
    assert!(
        narrow_text.contains("\n  const record = label;"),
        "two-space body: {narrow_text}"
    );
    assert!(
        wide_text.contains("\n    const record = label;"),
        "four-space body: {wide_text}"
    );
}

/// Verifies every exact resolved style re-parses to one semantic module.
#[test]
fn every_resolved_style_parses_back_to_one_semantic_module() {
    let model = sample_model();
    let styles = [
        value(EcmaStyle::default()),
        EcmaStyleAdapter
            .merge(
                &kind("javascript"),
                &value(EcmaStyle::default()),
                &raw("quote=single"),
            )
            .expect("quoting merges"),
        EcmaStyleAdapter
            .merge(
                &kind("typescript"),
                &value(EcmaStyle::default()),
                &raw("indent=tabs,indent-width=8"),
            )
            .expect("tab indentation merges"),
    ];

    for style in &styles {
        let resolved = ResolvedStyle::inline(style.clone());
        let rendered = render_with_style(&model, &decoded(&resolved)).expect("style render");
        let label = String::from_utf8(style.as_bytes().to_vec()).expect("style bytes are utf-8");
        assert_eq!(
            parse_source(Path::new("sample.js"), &rendered).expect("style render re-parses"),
            model,
            "style {label} changed semantic content"
        );
        assert_eq!(
            render_with_style(&model, &decoded(&resolved)).expect("style re-render"),
            rendered,
            "style {label} is not idempotent"
        );
    }
}

/// Verifies a resolved style feeds the renderer without further lookup.
#[test]
fn a_resolved_style_renders_without_further_lookup() {
    let model = sample_model();
    let resolved = ResolvedStyle::inline(raw("indent=tabs,indent-width=8,quote=double"));
    let rendered = render_with_style(&model, &decoded(&resolved)).expect("resolved render");

    assert!(
        rendered.contains("\n\t\ttotal = value;"),
        "tab-indented body: {rendered}"
    );
    assert!(
        !rendered.contains("\n  "),
        "the resolved style carries no space indentation: {rendered}"
    );
}
