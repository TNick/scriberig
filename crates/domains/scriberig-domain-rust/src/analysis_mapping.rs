//! Maps Rust syntax names to stable canonical entity identities.

use blake3::Hasher;
use scriberig_core::EntityId;
use std::collections::BTreeMap;

/// Stable mapping from source-level Rust names to canonical entities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisMapping {
    /// Canonical root entity for the parsed source.
    pub source: EntityId,
    /// Function and item names mapped to canonical entities.
    pub symbols: BTreeMap<String, EntityId>,
}

impl AnalysisMapping {
    /// Creates a deterministic mapping for named functions.
    ///
    /// # Parameters
    /// - `source`: Canonical source entity.
    /// - `names`: Function names in source order.
    ///
    /// # Returns
    /// A mapping whose IDs are derived from canonical source identity and name.
    #[must_use]
    pub fn for_functions(source: EntityId, names: impl IntoIterator<Item = String>) -> Self {
        let symbols = names
            .into_iter()
            .map(|name| {
                let mut hasher = Hasher::new();
                hasher.update(b"scriberig.rust.symbol.v1");
                hasher.update(source.as_bytes());
                hasher.update(name.as_bytes());
                let digest = hasher.finalize();
                let mut bytes = [0; 16];
                bytes.copy_from_slice(&digest.as_bytes()[..16]);
                (name, EntityId::from_bytes(bytes))
            })
            .collect();
        Self { source, symbols }
    }
}
