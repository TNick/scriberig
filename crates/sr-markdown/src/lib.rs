//! Provides deterministic parsing, rendering, and equivalence for Markdown projections.

pub mod adapter;
pub mod candidate;
pub mod diagnostic;
pub mod equivalence;
pub mod model;
pub mod parser;
pub mod render;
pub mod style;

pub use adapter::{MARKDOWN_KIND, MarkdownAdapter};
pub use candidate::CandidateDocument;
pub use equivalence::{equivalent, semantically_equivalent};
pub use model::{MarkdownBlock, MarkdownDocument, Section};
pub use parser::{parse, parse_document};
pub use render::{render, render_document};
pub use sr_projection::ProjectionAdapter;
pub use style::MarkdownStyle;
