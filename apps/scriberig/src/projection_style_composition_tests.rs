//! Verifies the product composition registers every domain style adapter so
//! application resolution reaches each source kind.

use sr_projection::ProjectionKind;

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

/// Verifies each registered source kind resolves its own domain style, and that
/// a kind no domain registered stays unsupported instead of receiving a generic
/// style.
#[test]
fn the_composition_registers_the_rust_and_ecma_source_kinds() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let configuration =
        super::domain_projection_style_configuration(&directory.path().join("scriberig.db"))
            .expect("the composition opens its style settings");

    for label in ["rust", "javascript", "typescript"] {
        let projection = kind(label);
        assert!(
            configuration.adapter.style_schema(&projection).is_some(),
            "the composition declares a style schema for {label}"
        );
        assert!(
            configuration.adapter.default_style(&projection).is_ok(),
            "the composition supplies an exact default style for {label}"
        );
    }

    let javascript = kind("javascript");
    let resolved = configuration
        .adapter
        .default_style(&javascript)
        .expect("the ECMAScript domain owns its default");
    let domain_default = scriberig_domain_ecma::EcmaStyle::default().encode();
    assert_eq!(
        resolved.value().as_bytes(),
        domain_default.as_slice(),
        "the composition resolves the ECMAScript domain default, not a generic style"
    );
    assert_eq!(
        configuration
            .adapter
            .style_schema(&javascript)
            .expect("the ECMAScript schema is declared")
            .reference
            .adapter_id(),
        "ecma-source-style"
    );

    let unregistered = kind("python");
    assert!(configuration.adapter.style_schema(&unregistered).is_none());
    assert_eq!(
        configuration
            .adapter
            .default_style(&unregistered)
            .expect_err("no domain registers python")
            .detail
            .code,
        "style-domain-not-registered"
    );
}
