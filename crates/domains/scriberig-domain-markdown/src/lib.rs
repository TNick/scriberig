//! Adapts Markdown documents to the universal semantic and projection architecture.

pub mod classification;
pub mod schema;
pub mod types;

pub use classification::{InformationClass, classify};
pub use schema::{OrderedRelation, markdown_schema};
