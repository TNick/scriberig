//! Provides deterministic parsing, rendering, and equivalence for Markdown projections.

pub mod adapter;
pub mod candidate;
pub mod diagnostic;
pub mod equivalence;
pub mod match_nodes;
pub mod model;
pub mod parser;
pub mod reconcile;
pub mod render;
pub mod style;

pub use adapter::{MARKDOWN_KIND, MarkdownAdapter};
pub use candidate::CandidateDocument;
pub use equivalence::{equivalent, semantically_equivalent};
pub use match_nodes::{CanonicalNode, MatchEvidence, MatchReport, NodeMatch, match_nodes};
pub use model::{MarkdownBlock, MarkdownDocument, Section};
pub use parser::{parse, parse_document};
pub use reconcile::{
    MarkdownMigrationProposal, MarkdownReconciliation, ReconciliationAmbiguity, reconcile,
};
pub use render::{render, render_document};
pub use sr_projection::ProjectionAdapter;
pub use style::MarkdownStyle;
