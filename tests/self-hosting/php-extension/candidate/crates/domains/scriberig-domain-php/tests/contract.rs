//! Verifies the PHP candidate adapter contract against golden source fixtures.

use std::path::Path;

use scriberig_domain_php::parse;

/// PHP-DOMAIN-01 through -04 verify declarations, nested syntax, comments, and inline HTML.
#[test]
fn parses_complete_php_and_mixed_inline_html() {
    let root = fixture_directory(
        "../../../../php-project/src",
        "../../../tests/self-hosting/php-extension/php-project/src",
    );
    let source = std::fs::read_to_string(root.join("Greeter.php")).expect("PHP fixture");
    let document = parse(Path::new("Greeter.php"), &source).expect("valid PHP");
    let kinds = document
        .nodes
        .iter()
        .map(|node| node.kind.as_str())
        .collect::<Vec<_>>();
    assert!(kinds.contains(&"class_declaration"));
    assert!(kinds.contains(&"method_declaration"));
    assert!(kinds.contains(&"return_statement"));
    assert!(
        document
            .nodes
            .iter()
            .any(|node| node.text.as_deref() == Some("// greeting comment"))
    );
    assert!(document.declarations.iter().any(|item| item.name == "App"));
    assert!(
        document
            .declarations
            .iter()
            .any(|item| item.name == "Greeter")
    );
    assert!(
        document
            .declarations
            .iter()
            .any(|item| item.name == "greet")
    );
    assert!(
        document
            .declarations
            .iter()
            .any(|item| item.name == "helper")
    );
    let mixed = std::fs::read_to_string(root.join("inline.php")).expect("inline fixture");
    let mixed_document = parse(Path::new("inline.php"), &mixed).expect("mixed PHP parses");
    assert!(mixed_document.nodes.iter().any(|node| node.kind == "text"));
    assert!(
        mixed_document
            .declarations
            .iter()
            .any(|item| item.name == "render")
    );
}

/// PHP-DOMAIN-05 and -06 verify malformed diagnostics and deterministic syntax.
#[test]
fn rejects_malformed_php_and_serializes_deterministically() {
    let root = fixture_directory(
        "../../../../php-project/src",
        "../../../tests/self-hosting/php-extension/php-project/src",
    );
    let broken_root = fixture_directory(
        "../../../../fixtures",
        "../../../tests/self-hosting/php-extension/fixtures",
    );
    let broken = std::fs::read_to_string(broken_root.join("Broken.php.txt"))
        .expect("broken fixture");
    let error = parse(Path::new("Broken.php"), &broken).expect_err("malformed PHP rejected");
    assert!(error.diagnostics.iter().any(|item| item.contains("syntax")));
    assert!(error.diagnostics.iter().any(|item| item.contains("bytes")));
    let source = std::fs::read_to_string(root.join("Greeter.php")).expect("PHP fixture");
    let first = parse(Path::new("Greeter.php"), &source).expect("first parse");
    let second = parse(Path::new("Greeter.php"), &source).expect("second parse");
    assert_eq!(first, second);
    assert!(
        first
            .nodes
            .iter()
            .all(|node| node.start_byte <= node.end_byte)
    );
    assert_eq!(first.nodes.first().map(|node| node.path.as_str()), Some(""));
    assert!(first.nodes.iter().skip(1).all(|node| !node.path.is_empty()));
}

/// Resolves checked-in fixtures from the standalone fixture or applied workspace layout.
///
/// # Parameters
/// - `fixture_relative`: Path from the standalone candidate crate location.
/// - `workspace_relative`: Path from the production package location.
///
/// # Returns
/// The first existing fixture directory.
fn fixture_directory(fixture_relative: &str, workspace_relative: &str) -> std::path::PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest.join(fixture_relative);
    if fixture.is_dir() {
        fixture
    } else {
        manifest.join(workspace_relative)
    }
}
