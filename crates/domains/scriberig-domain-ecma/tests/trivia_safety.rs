//! Verifies source trivia and syntax outside the semantic subset fail closed.

use std::path::Path;

use scriberig_domain_ecma::{EcmaStyle, parse_source, render, render_with_style};

/// Refuses comments rather than silently dropping them during rendering.
#[test]
fn comments_and_tooling_directives_are_reported_as_unsupported() {
    for source in [
        "// license header\nconst value = 1;",
        "/** Public API docs. */\nexport function read() { return value; }",
        "// eslint-disable-next-line no-console\nconsole.log(value);",
        "const value = 1; /* sourceMappingURL=main.js.map */",
    ] {
        let error = parse_source(Path::new("main.js"), source)
            .expect_err("unmodeled trivia must not disappear from an editable model");
        assert!(
            error
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("comment trivia")),
            "unexpected refusal for {source:?}: {error}"
        );
    }
}

/// Explicitly refuses TSX syntax instead of claiming the whole syntax mode.
#[test]
fn unsupported_tsx_constructs_return_a_structured_diagnostic() {
    let error = parse_source(
        Path::new("component.tsx"),
        "export const View = () => <section>ready</section>;",
    )
    .expect_err("JSX is outside the current semantic subset");

    assert!(error.fatal);
    assert!(
        error.diagnostics.iter().any(|diagnostic| {
            diagnostic.contains("unsupported ECMAScript construct")
                || diagnostic.contains("syntax error")
        }),
        "TSX refusal has a structured diagnostic: {error}"
    );
}

/// Supports and round-trips a basic TypeScript source file without type syntax.
#[test]
fn supported_typescript_subset_renders_deterministically() {
    let model = parse_source(Path::new("main.ts"), "export const value = 1;")
        .expect("the TypeScript subset parses");
    let style = EcmaStyle::default();
    let rendered = render_with_style(&model, &style).expect("the TypeScript subset renders");

    assert_eq!(render(&model).expect("default rendering"), rendered);
    assert_eq!(
        parse_source(Path::new("main.ts"), &rendered).expect("rendered TypeScript parses"),
        model
    );
}
