//! Produces derived Rust symbol, call, type, and diagnostic facts.

use crate::error::RustAnalysisError;
use crate::{AnalysisMapping, AnalysisProfile, DiagnosticSeverity, RustDiagnostic};
use quote::ToTokens;
use scriberig_core::{EntityId, RevisionId};
use scriberig_derive::{DerivationKey, DerivedResultHandle, DerivedResultStatus};
use std::collections::BTreeMap;
use syn::{Expr, ExprCall, Item, ReturnType, Type, visit::Visit};

/// A resolved source symbol reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolReference {
    /// Canonical referencing function entity.
    pub from: EntityId,
    /// Canonical referenced function entity.
    pub to: EntityId,
    /// Source-level name used for resolution.
    pub name: String,
}

/// A resolved function call relation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallRelation {
    /// Canonical caller entity.
    pub caller: EntityId,
    /// Canonical callee entity.
    pub callee: EntityId,
}

/// A type fact mapped to a canonical function entity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeFact {
    /// Canonical entity whose type was resolved.
    pub entity: EntityId,
    /// Rust type syntax resolved by the analysis.
    pub type_text: String,
}

/// Provenance retained for one derived analysis result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisProvenance {
    /// Stable producer identity.
    pub producer: String,
    /// Exact toolchain/profile inputs.
    pub profile: AnalysisProfile,
    /// Exact source revision dependency.
    pub source_revision: RevisionId,
}

/// Immutable derived Rust analysis output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustAnalysis {
    /// Exact derivation request identity.
    pub key: DerivationKey,
    /// Immutable result artifact identity.
    pub handle: DerivedResultHandle,
    /// Structured success or failure status.
    pub status: DerivedResultStatus,
    /// Canonical source-to-symbol mapping.
    pub mapping: AnalysisMapping,
    /// Resolved symbol references.
    pub references: Vec<SymbolReference>,
    /// Caller/callee relations.
    pub calls: Vec<CallRelation>,
    /// Resolved type facts.
    pub types: Vec<TypeFact>,
    /// Compiler and lint diagnostics.
    pub diagnostics: Vec<RustDiagnostic>,
    /// Exact producer and input provenance.
    pub provenance: AnalysisProvenance,
}

impl RustAnalysis {
    /// Returns references targeting one canonical symbol.
    ///
    /// # Parameters
    /// - `target`: Canonical symbol entity.
    ///
    /// # Returns
    /// Matching references in deterministic source order.
    #[must_use]
    pub fn references_to(&self, target: EntityId) -> Vec<&SymbolReference> {
        self.references
            .iter()
            .filter(|reference| reference.to == target)
            .collect()
    }

    /// Returns canonical callers of one symbol.
    ///
    /// # Parameters
    /// - `callee`: Canonical callee entity.
    ///
    /// # Returns
    /// Caller entities in analysis order.
    #[must_use]
    pub fn callers_of(&self, callee: EntityId) -> Vec<EntityId> {
        self.calls
            .iter()
            .filter(|call| call.callee == callee)
            .map(|call| call.caller)
            .collect()
    }

    /// Returns diagnostics affecting one canonical entity.
    ///
    /// # Parameters
    /// - `entity`: Entity whose diagnostics are requested.
    ///
    /// # Returns
    /// Matching derived diagnostics.
    #[must_use]
    pub fn diagnostics_for(&self, entity: EntityId) -> Vec<&RustDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.entity == Some(entity))
            .collect()
    }
}

/// Runs deterministic static analysis over canonical Rust source.
///
/// # Parameters
/// - `source`: Canonical Rust source text.
/// - `source_entity`: Canonical source entity identity.
/// - `source_revision`: Exact canonical syntax revision.
/// - `profile`: Compiler/profile inputs affecting semantics.
///
/// # Returns
/// Immutable derived facts with exact dependency and provenance metadata.
///
/// # Errors
/// Returns a structured parse failure. Canonical source is never changed.
pub fn analyze(
    source: &str,
    source_entity: EntityId,
    source_revision: RevisionId,
    profile: &AnalysisProfile,
) -> Result<RustAnalysis, RustAnalysisError> {
    let file = syn::parse_file(source).map_err(|error| RustAnalysisError {
        code: "parse".to_owned(),
        message: error.to_string(),
    })?;
    let names: Vec<String> = file.items.iter().filter_map(function_name).collect();
    let mapping = AnalysisMapping::for_functions(source_entity, names);
    let mut references = Vec::new();
    let mut calls = Vec::new();
    let mut types = Vec::new();
    for item in &file.items {
        let Item::Fn(function) = item else { continue };
        let Some(&caller) = mapping.symbols.get(function.sig.ident.to_string().as_str()) else {
            continue;
        };
        if let ReturnType::Type(_, ty) = &function.sig.output {
            types.push(TypeFact {
                entity: caller,
                type_text: type_text(ty),
            });
        }
        let mut visitor = CallVisitor {
            mapping: &mapping.symbols,
            caller,
            references: &mut references,
            calls: &mut calls,
        };
        visitor.visit_block(&function.block);
    }
    let key = DerivationKey::new(
        profile.derivation_profile(),
        profile.dependencies(source_entity, source_revision),
    );
    let handle = DerivedResultHandle {
        artifact_id: analysis_id(source_entity, source_revision, profile),
    };
    Ok(RustAnalysis {
        key,
        handle,
        status: DerivedResultStatus::Succeeded,
        mapping,
        references,
        calls,
        types,
        diagnostics: Vec::new(),
        provenance: AnalysisProvenance {
            producer: format!("rustc:{}", profile.toolchain),
            profile: profile.clone(),
            source_revision,
        },
    })
}

/// Extracts a function name from a top-level item.
///
/// # Parameters
/// - `item`: Syntax item to inspect.
///
/// # Returns
/// Function name when the item is a function.
fn function_name(item: &Item) -> Option<String> {
    match item {
        Item::Fn(function) => Some(function.sig.ident.to_string()),
        _ => None,
    }
}
/// Renders a syntax type into a deterministic display representation.
///
/// # Parameters
/// - `ty`: Syntax type to render.
///
/// # Returns
/// Canonical token text for the type.
fn type_text(ty: &Type) -> String {
    ty.to_token_stream().to_string()
}
/// Derives an immutable result identity from all relevant analysis inputs.
///
/// # Parameters
/// - `source`: Canonical source entity.
/// - `revision`: Exact source revision.
/// - `profile`: Analysis environment.
///
/// # Returns
/// Content-derived result revision identity.
fn analysis_id(source: EntityId, revision: RevisionId, profile: &AnalysisProfile) -> RevisionId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig.rust.analysis.v1");
    hasher.update(source.as_bytes());
    hasher.update(revision.as_bytes());
    hasher.update(profile.toolchain.as_bytes());
    hasher.update(profile.configuration_id().as_bytes());
    RevisionId::from_bytes(*hasher.finalize().as_bytes())
}

/// Visits one function body and resolves calls to known canonical functions.
///
/// # Private members
/// - `mapping`: Source names to canonical IDs.
/// - `caller`: Current canonical caller.
/// - `references`: Collected symbol references.
/// - `calls`: Collected caller/callee relations.
struct CallVisitor<'a> {
    mapping: &'a BTreeMap<String, EntityId>,
    caller: EntityId,
    references: &'a mut Vec<SymbolReference>,
    calls: &'a mut Vec<CallRelation>,
}
impl<'ast> Visit<'ast> for CallVisitor<'_> {
    /// Resolves a call expression when its path names a known function.
    ///
    /// # Parameters
    /// - `call`: Call expression being visited.
    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(path) = &*call.func {
            if let Some(name) = path.path.get_ident().map(ToString::to_string) {
                if let Some(&to) = self.mapping.get(&name) {
                    self.references.push(SymbolReference {
                        from: self.caller,
                        to,
                        name,
                    });
                    self.calls.push(CallRelation {
                        caller: self.caller,
                        callee: to,
                    });
                }
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
}

/// Builds a failed derived result with an explicit diagnostic.
///
/// # Parameters
/// - `error`: Structured analysis failure.
/// - `source_entity`: Canonical source entity.
/// - `source_revision`: Exact source revision.
/// - `profile`: Analysis environment.
///
/// # Returns
/// Failed analysis facts that preserve canonical identity and provenance.
#[must_use]
pub fn failed_analysis(
    error: RustAnalysisError,
    source_entity: EntityId,
    source_revision: RevisionId,
    profile: &AnalysisProfile,
) -> RustAnalysis {
    let key = DerivationKey::new(
        profile.derivation_profile(),
        profile.dependencies(source_entity, source_revision),
    );
    RustAnalysis {
        key,
        handle: DerivedResultHandle {
            artifact_id: analysis_id(source_entity, source_revision, profile),
        },
        status: DerivedResultStatus::FailedDeterministic,
        mapping: AnalysisMapping::for_functions(source_entity, Vec::<String>::new()),
        references: Vec::new(),
        calls: Vec::new(),
        types: Vec::new(),
        diagnostics: vec![RustDiagnostic {
            severity: DiagnosticSeverity::Error,
            code: Some(error.code),
            message: error.message,
            entity: Some(source_entity),
        }],
        provenance: AnalysisProvenance {
            producer: format!("rustc:{}", profile.toolchain),
            profile: profile.clone(),
            source_revision,
        },
    }
}
