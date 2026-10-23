//! Verifies the ECMAScript rendering foundation: determinism, exact style
//! consumption, model round-tripping, and structured refusals.

use std::path::Path;

use scriberig_domain_ecma::source::EcmaSource;
use scriberig_domain_ecma::{
    EcmaIndent, EcmaQuote, EcmaStyle, parse_source, render, render_with_style,
};

/// Bounded-subset module exercising every supported construct.
const SAMPLE: &str = "\
import './side-effects.js';
import ready from 'state';
import { alpha, beta as gamma } from 'pkg/mod';
export const threshold = 1.50;
export let total = count(alpha);
var scratch = null;
export function gather(value, label) {
  const record = label;
  log(record, threshold);
  if (value) {
    total = value;
  } else if (label) {
    total = gamma;
  } else {
    total = 'none';
  }
  return record;
}
export class Ledger {
  constructor(start) {
    saved = start;
  }
  load(key) {
    return read(key);
  }
  save(key, value) {
    write(key, value);
    if (key) {
      ready(value);
    } else {
      ready(scratch);
    }
  }
}
const quoted = \"she said \\\"hi\\\"\\nand\\ttab\";
const mapped = items.map((item) => item.value);
const handler = (value) => {
  counter = value;
  return counter;
};
const idle = () => {};
if (ready) {
  done(mapped, handler);
}
done(process(idle), threshold);
";

/// Parses the sample module with a fixed path.
///
/// # Returns
/// The semantic model of the supported subset.
fn sample_model() -> EcmaSource {
    parse_source(Path::new("sample.js"), SAMPLE).expect("sample is in the supported subset")
}

/// Returns one supported style per indentation and quote choice.
///
/// # Returns
/// Every style the renderer implements.
fn supported_styles() -> Vec<EcmaStyle> {
    let mut styles = Vec::new();
    for width in [2u8, 4, 8] {
        styles.push(EcmaStyle {
            indent: EcmaIndent::Spaces,
            indent_width: width,
            quote: EcmaQuote::Double,
        });
        styles.push(EcmaStyle {
            indent: EcmaIndent::Spaces,
            indent_width: width,
            quote: EcmaQuote::Single,
        });
    }
    styles.push(EcmaStyle {
        indent: EcmaIndent::Tabs,
        indent_width: 2,
        quote: EcmaQuote::Double,
    });
    styles
}

/// The default style is the canonical two-space double-quote rendering.
#[test]
fn default_style_is_the_canonical_rendering() {
    let style = EcmaStyle::default();
    assert_eq!(style.indent, EcmaIndent::Spaces);
    assert_eq!(style.indent_width, 2);
    assert_eq!(style.quote, EcmaQuote::Double);
    assert_eq!(
        String::from_utf8(style.encode()).expect("canonical bytes are utf-8"),
        "indent=spaces,indent-width=2,quote=double"
    );
    assert_eq!(
        EcmaStyle::decode(&style.encode()).expect("canonical decode"),
        style
    );
}

/// The same model and style always render the same bytes.
#[test]
fn rendering_is_deterministic() {
    let style = EcmaStyle::default();
    let first = render_with_style(&sample_model(), &style).expect("renders");
    let second = render_with_style(&sample_model(), &style).expect("renders");
    assert_eq!(first, second);
    let model = sample_model();
    assert_eq!(
        render_with_style(&model, &style).expect("renders"),
        render_with_style(&model, &style).expect("renders")
    );
    assert_eq!(render(&model).expect("canonical render"), first);
}

/// Two supported styles produce visibly different bytes for one model.
#[test]
fn two_styles_render_different_bytes() {
    let model = sample_model();
    let narrow = EcmaStyle {
        indent: EcmaIndent::Spaces,
        indent_width: 2,
        quote: EcmaQuote::Double,
    };
    let wide = EcmaStyle {
        indent: EcmaIndent::Spaces,
        indent_width: 4,
        quote: EcmaQuote::Double,
    };
    let single = EcmaStyle {
        indent: EcmaIndent::Spaces,
        indent_width: 2,
        quote: EcmaQuote::Single,
    };
    let narrow_text = render_with_style(&model, &narrow).expect("two-space render");
    let wide_text = render_with_style(&model, &wide).expect("four-space render");
    let single_text = render_with_style(&model, &single).expect("single-quote render");
    assert_ne!(narrow_text, wide_text);
    assert_ne!(narrow_text, single_text);
    assert!(narrow_text.contains("\n  const record = label;"));
    assert!(wide_text.contains("\n    const record = label;"));
    assert!(single_text.contains("import 'pkg/mod'") || single_text.contains("from 'pkg/mod'"));
    assert!(narrow_text.contains("from \"pkg/mod\""));
}

/// Every supported style re-parses to the identical semantic model.
#[test]
fn every_supported_style_round_trips_identically() {
    let model = sample_model();
    for style in supported_styles() {
        let text = render_with_style(&model, &style).expect("style renders");
        let reparsed =
            parse_source(Path::new("sample.js"), &text).expect("rendered text re-parses");
        assert_eq!(model, reparsed, "style {style:?} changed the model");
        assert_eq!(
            render_with_style(&reparsed, &style).expect("re-render"),
            text,
            "style {style:?} is not idempotent"
        );
    }
}

/// An unsupported construct is refused with a structured error naming it.
#[test]
fn refuses_constructs_outside_the_subset() {
    let error = parse_source(
        Path::new("switch.js"),
        "export function pick(value) { switch (value) { case 1: return value; } }",
    )
    .expect_err("switch is outside the subset");
    assert!(error.fatal);
    assert!(
        error
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic
                .contains("unsupported ECMAScript construct: switch statement")),
        "{error}"
    );

    let typed = parse_source(Path::new("typed.ts"), "export const value: number = 1;")
        .expect_err("type annotations are outside the subset");
    assert!(
        typed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("typed variable declaration")),
        "{typed}"
    );

    let binary = parse_source(Path::new("binary.js"), "const sum = a + b;")
        .expect_err("binary expressions are outside the subset");
    assert!(
        binary
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("binary expression")),
        "{binary}"
    );
}

/// A named import whose exported name is a string literal is refused.
///
/// Rendering such a name unquoted emits invalid ECMAScript, and re-parsing the
/// rendered text cannot reproduce the model, so the construct is refused
/// instead of approximated.
#[test]
fn refuses_string_literal_import_names() {
    let error = parse_source(
        Path::new("named.js"),
        "import { \"foo-bar\" as foo } from \"pkg\";",
    )
    .expect_err("string-literal import names are outside the subset");
    assert!(error.fatal);
    assert!(
        error
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("string-literal import name")),
        "{error}"
    );
}

/// Constructs that can appear inside supported nodes are refused by name.
#[test]
fn refuses_constructs_nested_in_supported_nodes() {
    let cases = [
        (
            "chain.js",
            "const value = source?.field;",
            "optional chain expression",
        ),
        (
            "decorated.ts",
            "@logged\nclass Ledger {\n  load() {}\n}\n",
            "class heritage or decorated class",
        ),
        (
            "strict.js",
            "\"use strict\";\nconst ready = true;",
            "directive",
        ),
        (
            "strict-fn.js",
            "function load() {\n  \"use strict\";\n  return ready;\n}",
            "directive",
        ),
        (
            "specifiers.js",
            "const ready = true;\nexport { ready };",
            "export specifier list",
        ),
        (
            "template.js",
            "const message = `ready ${flag}`;",
            "unsupported ECMAScript construct: template literal",
        ),
        (
            "destructure.js",
            "const { count } = counter;",
            "object binding pattern",
        ),
        (
            "array-pattern.js",
            "const [first] = items;",
            "array binding pattern",
        ),
        ("compound.js", "counter += 1;", "compound assignment +="),
        ("newable.js", "const store = new Map();", "new expression"),
        (
            "getter.js",
            "class Ledger {\n  get size() {\n    return count;\n  }\n}",
            "class getter or setter",
        ),
        (
            "static-member.js",
            "class Ledger {\n  static open() {}\n}",
            "static or computed class member",
        ),
    ];
    for (name, source, fragment) in cases {
        let error =
            parse_source(Path::new(name), source).expect_err("{source} is outside the subset");
        assert!(error.fatal, "{name} reported a non-fatal refusal");
        assert!(
            error
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains(fragment)),
            "{name} refused with {error}, expected a diagnostic containing {fragment}"
        );
    }
}

/// An unsupported style value is refused with an error naming the field.
#[test]
fn refuses_unsupported_style_values() {
    let style = EcmaStyle {
        indent: EcmaIndent::Spaces,
        indent_width: 3,
        quote: EcmaQuote::Double,
    };
    let reason = style
        .validate()
        .expect_err("three-space indent is unsupported");
    assert!(reason.contains("indent-width"), "{reason}");
    let error = render_with_style(&sample_model(), &style).expect_err("render refuses the style");
    assert!(error.fatal);
    assert!(
        error
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("indent-width")),
        "{error}"
    );
    let unknown =
        EcmaStyle::decode(b"indent=spaces,indent-width=2,quote=backtick").expect_err("bad quote");
    assert!(unknown.contains("quote"), "{unknown}");
    let missing = EcmaStyle::decode(b"semicolons=always").expect_err("unknown field");
    assert!(missing.contains("semicolons"), "{missing}");
}

/// Style decoding accepts canonical bytes only, never a repeated field.
#[test]
fn refuses_duplicate_style_fields() {
    let duplicate = EcmaStyle::decode(b"indent=spaces,indent-width=2,quote=double,quote=single")
        .expect_err("a repeated field is not canonical");
    assert!(duplicate.contains("quote"), "{duplicate}");
    assert!(duplicate.contains("duplicate"), "{duplicate}");
    let repeated_width = EcmaStyle::decode(b"indent-width=2,indent-width=4")
        .expect_err("a repeated field is not canonical");
    assert!(repeated_width.contains("indent-width"), "{repeated_width}");
    let canonical = EcmaStyle::decode(b"indent=tabs,indent-width=8,quote=single")
        .expect("canonical bytes decode");
    assert_eq!(canonical.indent, EcmaIndent::Tabs);
    assert_eq!(canonical.indent_width, 8);
    assert_eq!(canonical.quote, EcmaQuote::Single);
}
