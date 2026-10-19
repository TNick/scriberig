//! Verifies derived Rust compiler-analysis facts and provenance.

use scriberig_core::{EntityId, RevisionId};
use scriberig_derive::{Dependency, DerivedResultStatus};
use scriberig_domain_rust::{AnalysisProfile, DiagnosticSeverity, analyze, failed_analysis};

/// Creates a deterministic compiler profile fixture.
///
/// A profile with deterministic toolchain and configuration inputs.
/// # Returns
/// The operation result.
fn profile() -> AnalysisProfile {
    AnalysisProfile::new(
        "1.84.0",
        "x86_64-unknown-linux-gnu",
        vec!["std".to_owned()],
        "workspace-v1",
        "check-cfg-v1",
    )
}

/// Verifies function references and caller/callee relations use canonical IDs.
#[test]
fn references_resolve_to_canonical_symbols() {
    let source = "fn callee() -> i32 { 1 } fn caller() -> i32 { callee() }";
    let source_entity = EntityId::from_bytes([1; 16]);
    let result =
        analyze(source, source_entity, RevisionId::new([2; 32]), &profile()).expect("analysis");
    let target_function = result.mapping.symbols["callee"];
    let source_function = result.mapping.symbols["caller"];
    assert_eq!(result.calls.len(), 1);
    assert_eq!(result.calls[0].caller, source_function);
    assert_eq!(result.calls[0].callee, target_function);
    assert_eq!(result.references[0].to, target_function);
    assert_eq!(result.callers_of(target_function), vec![source_function]);
    assert_eq!(result.references_to(target_function).len(), 1);
}

/// Verifies type facts are derived separately from canonical syntax.
#[test]
fn type_facts_and_exact_dependencies_are_recorded() {
    let source_entity = EntityId::from_bytes([3; 16]);
    let revision = RevisionId::new([4; 32]);
    let result = analyze(
        "fn answer() -> String { String::new() }",
        source_entity,
        revision,
        &profile(),
    )
    .expect("analysis");
    assert_eq!(result.types[0].type_text, "String");
    assert!(
        result
            .key
            .dependencies
            .as_slice()
            .contains(&Dependency::ExactRevision {
                entity: source_entity,
                revision
            })
    );
    assert_eq!(result.provenance.source_revision, revision);
    assert_eq!(result.status, DerivedResultStatus::Succeeded);
}

/// Verifies parse failures remain derived diagnostics and preserve source identity.
#[test]
fn failed_analysis_does_not_replace_canonical_source() {
    let source_entity = EntityId::from_bytes([5; 16]);
    let revision = RevisionId::new([6; 32]);
    let failed = failed_analysis(
        scriberig_domain_rust::RustAnalysisError {
            code: "parse".to_owned(),
            message: "invalid".to_owned(),
        },
        source_entity,
        revision,
        &profile(),
    );
    assert_eq!(failed.status, DerivedResultStatus::FailedDeterministic);
    assert_eq!(failed.diagnostics[0].severity, DiagnosticSeverity::Error);
    assert_eq!(failed.diagnostics[0].entity, Some(source_entity));
    assert_eq!(failed.provenance.source_revision, revision);
}

/// Verifies environment and source changes produce distinct derived handles.
#[test]
fn relevant_inputs_invalidate_analysis_identity() {
    let entity = EntityId::from_bytes([7; 16]);
    let first =
        analyze("fn f() {}", entity, RevisionId::new([8; 32]), &profile()).expect("analysis");
    let second =
        analyze("fn f() {}", entity, RevisionId::new([9; 32]), &profile()).expect("analysis");
    let alternate = AnalysisProfile::new(
        "1.85.0",
        "x86_64-unknown-linux-gnu",
        vec!["std".to_owned()],
        "workspace-v1",
        "check-cfg-v1",
    );
    let third =
        analyze("fn f() {}", entity, RevisionId::new([8; 32]), &alternate).expect("analysis");
    assert_ne!(first.handle, second.handle);
    assert_ne!(first.handle, third.handle);
}
