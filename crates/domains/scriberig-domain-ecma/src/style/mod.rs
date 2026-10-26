//! Declares the exact ECMAScript source rendering style owned by this domain
//! and the projection style adapter that exposes it to the shared layer.
//!
//! The style changes presentation only. Every supported value is implemented by
//! [`crate::render`], so selecting one never changes the semantic source model.
//! The `model` module owns the field set, canonical encoding, and validation;
//! the `adapter` module only delegates to them, which keeps style meaning
//! inside the domain and out of the shared resolution layer.

mod adapter;
mod model;

/// Projection kinds whose source text this domain renders.
///
/// These are the adapter identifiers `scriberig-project` selects from a file
/// extension, so a style adapter registered for them covers every ECMAScript
/// source this domain can parse.
pub const ECMA_SOURCE_KINDS: &[&str] = &["javascript", "typescript"];

pub use adapter::EcmaStyleAdapter;
pub use model::{EcmaIndent, EcmaQuote, EcmaStyle};
