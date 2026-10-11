//! Adapts Rust source syntax to the shared language-independent code domain.

pub mod analysis;
pub mod analysis_mapping;
pub mod analysis_profile;
pub mod code_groups;
pub mod comments;
pub mod diagnostics;
pub mod error;
pub mod mapping;
pub mod parse;
pub mod render;
pub mod schema;
pub mod syntax;

pub use analysis::{
    AnalysisProvenance, CallRelation, RustAnalysis, SymbolReference, TypeFact, analyze,
    failed_analysis,
};
pub use analysis_mapping::AnalysisMapping;
pub use analysis_profile::AnalysisProfile;
pub use diagnostics::{DiagnosticSeverity, RustDiagnostic};
pub use error::{RustAnalysisError, RustSyntaxError};
pub use parse::parse;
pub use render::render;
pub use schema::{RUST_ENTITY_TYPES, RUST_RELATIONS};
pub use syntax::{
    RustBlock, RustDeclaration, RustFile, RustFunction, RustItem, RustMacro, RustParameter,
    RustStatement,
};
