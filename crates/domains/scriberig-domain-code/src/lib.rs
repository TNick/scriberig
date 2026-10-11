//! Defines language-independent code concepts above the universal semantic graph.
pub mod classification;
pub mod code_group;
pub mod comment;
pub mod schema;
pub mod types;
pub use classification::{InformationClass, classify};
pub use code_group::CodeGroup;
pub use comment::{Comment, CommentKind};
pub use schema::code_schema;
