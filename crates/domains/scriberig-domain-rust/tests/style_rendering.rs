//! Proves Rust source rendering is style-aware without changing semantic identity.

use scriberig_domain_rust::{
    RustIndent, RustStyle, RustStyleAdapter, parse, render, render_with_style,
};
use sr_projection::{
    ProjectionFidelity, ProjectionKind, ProjectionPurpose, ProjectionStyleAdapter, ResolvedStyle,
    StyleValue,
};

/// Source used by every style case.
const SOURCE: &str =
    "fn add(a: u64, b: u64) -> u64 {\n    a + b\n}\n\nfn zero() -> u64 {\n    0\n}\n";

/// Builds the Rust source projection kind.
///
/// # Returns
/// The kind this domain renders.
fn rust_kind() -> ProjectionKind {
    ProjectionKind::new(scriberig_domain_rust::RUST_SOURCE_KIND).expect("kind label is valid")
}

/// Encodes one style in the domain's canonical form.
///
/// - `style`: Style to encode.
///
/// # Returns
/// The exact adapter-owned style value.
fn value(style: &RustStyle) -> StyleValue {
    StyleValue::new(style.encode())
}

/// Verifies the canonical style reproduces the pre-style renderer.
#[test]
fn the_canonical_style_matches_the_existing_renderer() {
    let file = parse(SOURCE).expect("source parses");

    assert_eq!(
        render_with_style(&file, &RustStyle::default()).expect("canonical render"),
        render(&file).expect("existing render")
    );
}

/// Verifies two exact styles render one semantic file as different text.
#[test]
fn two_exact_styles_render_the_same_semantic_file_differently() {
    let file = parse(SOURCE).expect("source parses");
    let two_spaces = RustStyle {
        indent: RustIndent::Spaces,
        indent_width: 2,
        blank_lines_between_items: 1,
    };
    let tabs = RustStyle {
        indent: RustIndent::Tabs,
        indent_width: 4,
        blank_lines_between_items: 0,
    };

    let compact = render_with_style(&file, &two_spaces).expect("two-space render");
    let tabbed = render_with_style(&file, &tabs).expect("tab render");

    assert_ne!(compact, tabbed);
    assert!(compact.contains("\n  a + b"), "two-space body: {compact}");
    assert!(tabbed.contains("\n\ta + b"), "tab body: {tabbed}");
    assert!(
        !tabbed.contains("\n\nfn zero"),
        "tabs requested no blank line"
    );
}

/// Verifies each rendering parses back to the identical semantic file.
#[test]
fn every_supported_style_parses_back_to_one_semantic_file() {
    let file = parse(SOURCE).expect("source parses");
    let styles = [
        RustStyle::default(),
        RustStyle {
            indent: RustIndent::Spaces,
            indent_width: 8,
            blank_lines_between_items: 2,
        },
        RustStyle {
            indent: RustIndent::Tabs,
            indent_width: 4,
            blank_lines_between_items: 0,
        },
    ];

    for style in &styles {
        let rendered = render_with_style(&file, style).expect("style render");
        assert_eq!(
            parse(&rendered).expect("style render re-parses"),
            file,
            "style {} changed semantic content",
            String::from_utf8_lossy(&style.encode())
        );
    }
}

/// Verifies an unsupported style value is refused rather than approximated.
#[test]
fn the_renderer_and_adapter_refuse_an_unsupported_style_value() {
    let file = parse(SOURCE).expect("source parses");
    let unsupported = RustStyle {
        indent: RustIndent::Spaces,
        indent_width: 3,
        blank_lines_between_items: 1,
    };

    assert!(render_with_style(&file, &unsupported).is_err());
    let diagnostic =
        ProjectionStyleAdapter::normalize(&RustStyleAdapter, &rust_kind(), &value(&unsupported))
            .expect_err("the domain rejects width 3");

    assert_eq!(diagnostic.detail.code, "rust-style-value-unsupported");
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

/// Verifies a higher source overrides only the fields it names.
#[test]
fn the_adapter_merges_individual_style_fields() {
    let merged = ProjectionStyleAdapter::merge(
        &RustStyleAdapter,
        &rust_kind(),
        &value(&RustStyle::default()),
        &StyleValue::new(b"indent-width=2".to_vec()),
    )
    .expect("the domain merges a single field");

    assert_eq!(
        RustStyle::decode(merged.as_bytes()).expect("merged style decodes"),
        RustStyle {
            indent: RustIndent::Spaces,
            indent_width: 2,
            blank_lines_between_items: 1,
        }
    );
}

/// Verifies an unknown field is rejected with a structured diagnostic.
#[test]
fn the_adapter_rejects_an_unknown_style_field() {
    let diagnostic = ProjectionStyleAdapter::normalize(
        &RustStyleAdapter,
        &rust_kind(),
        &StyleValue::new(b"brace-style=next-line".to_vec()),
    )
    .expect_err("the renderer does not implement brace style");

    assert_eq!(diagnostic.detail.code, "rust-style-field-rejected");
    assert!(
        diagnostic
            .message
            .clone()
            .unwrap_or_default()
            .contains("brace-style"),
        "diagnostic: {:?}",
        diagnostic.message
    );
}

/// Verifies the declared default style is the domain default and is editable.
#[test]
fn the_declared_default_style_is_the_domain_default() {
    let default = ProjectionStyleAdapter::default_style(&RustStyleAdapter, &rust_kind())
        .expect("the domain declares a default style");
    let fidelity = ProjectionStyleAdapter::classify(
        &RustStyleAdapter,
        &rust_kind(),
        &default,
        ProjectionPurpose::SharedMaterialization,
    );

    assert_eq!(
        RustStyle::decode(default.value().as_bytes()).expect("default decodes"),
        RustStyle::default()
    );
    assert_eq!(fidelity.fidelity, ProjectionFidelity::LosslessEditable);
}

/// Verifies a style the renderer cannot read is classified read-only.
#[test]
fn a_style_the_renderer_cannot_read_is_read_only() {
    let fidelity = ProjectionStyleAdapter::classify(
        &RustStyleAdapter,
        &rust_kind(),
        &ResolvedStyle::inline(StyleValue::new(b"indent=elastic".to_vec())),
        ProjectionPurpose::InteractiveView,
    );

    assert_eq!(fidelity.fidelity, ProjectionFidelity::ReadOnlyLossy);
    assert!(!fidelity.fidelity.is_editable());
}

/// Verifies the schema is declared only for the kind this domain renders.
#[test]
fn the_schema_is_declared_only_for_rust_source() {
    let schema = ProjectionStyleAdapter::style_schema(&RustStyleAdapter, &rust_kind())
        .expect("the domain declares a style schema");
    let other = ProjectionKind::new("markdown").expect("kind label is valid");

    assert_eq!(schema.properties.len(), 3);
    assert_eq!(
        schema
            .properties
            .iter()
            .map(|p| p.key.as_str())
            .collect::<Vec<_>>(),
        vec!["indent", "indent-width", "blank-lines"]
    );
    assert!(ProjectionStyleAdapter::style_schema(&RustStyleAdapter, &other).is_none());
}

/// Verifies a resolved style feeds the renderer without further lookup.
#[test]
fn a_resolved_style_renders_without_further_lookup() {
    let file = parse(SOURCE).expect("source parses");
    let resolved = ResolvedStyle::inline(value(&RustStyle {
        indent: RustIndent::Spaces,
        indent_width: 2,
        blank_lines_between_items: 1,
    }));
    let style = RustStyle::decode(resolved.value().as_bytes()).expect("resolved style decodes");

    let rendered = render_with_style(&file, &style).expect("resolved render");
    assert_eq!(
        rendered
            .lines()
            .filter(|line| line.starts_with(' '))
            .collect::<Vec<_>>(),
        vec!["  a + b", "  0"],
        "the resolved style reaches the renderer with no further lookup"
    );
}
