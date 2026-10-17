//! Resolves and renders hierarchical CLI configuration.

mod credentials;
mod files;
mod render;
mod resolver;

pub use render::{render, source_designator};
pub use resolver::{ConfigEntry, ConfigError, ConfigResolver, ConfigSource, ResolvedConfig};
