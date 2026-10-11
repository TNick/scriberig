//! Adapts Rust source syntax to the shared language-independent code domain.

pub mod code_groups;
pub mod comments;
pub mod error;
pub mod mapping;
pub mod parse;
pub mod render;
pub mod schema;
pub mod syntax;

pub use error::RustSyntaxError;
pub use parse::parse;
pub use render::render;
pub use schema::{RUST_ENTITY_TYPES, RUST_RELATIONS};
pub use syntax::{
    RustBlock, RustDeclaration, RustFile, RustFunction, RustItem, RustMacro, RustParameter,
    RustStatement,
};
