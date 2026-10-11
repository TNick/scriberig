//! Tests Rust syntax parsing and deterministic rendering.
use scriberig_domain_rust::{RustItem, parse, render};

/// Parses functions, attributes, patterns, and macros into semantic syntax.
#[test]
fn parses_supported_rust_syntax() {
    let source = "#[inline] fn add((a, b): (i32, i32)) -> i32 { let value = a + b; value } macro_rules! demo { () => {} }";
    let file = parse(source).expect("parse");
    assert!(matches!(file.items.first(), Some(RustItem::Function(_))));
    assert!(
        file.items
            .iter()
            .any(|item| matches!(item, RustItem::Macro(_)))
    );
    let rendered = render(&file).expect("render");
    assert!(rendered.contains("add"));
    assert!(rendered.contains("macro_rules"));
}

/// Formatting-only whitespace does not alter parsed semantic structure.
#[test]
fn formatting_does_not_change_function_structure() {
    let left = parse("fn value(x: i32) { x; }").expect("parse");
    let right = parse("fn value( x : i32 ){ x ; }").expect("parse");
    assert_eq!(left.items, right.items);
}
