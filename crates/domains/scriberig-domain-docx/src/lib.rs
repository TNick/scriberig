//! Imports DOCX paragraph and table semantics without claiming write fidelity.

mod edit;
mod import;
mod reconciliation;
mod relationships;
mod styles;

pub use edit::{append_simple_paragraph, replace_simple_paragraph, validate_package};
pub use import::{DocxImport, import_docx, semantic_document_id};
pub use reconciliation::{
    DocxReconciliationOutcome, materialize_reconciled_docx, reconcile_external_docx,
};
