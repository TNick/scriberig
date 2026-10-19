//! Maintains secret-free authorization metadata in protected storage.

mod authorized;
mod lifecycle;
mod storage;

use crate::SecretPolicy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub use authorized::{AuthorizedCredential, resolve_authorized};
pub use storage::{RegistryGuard, RegistryLocations};

/// Records only managed aliases and explicit authorization bindings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    /// Registry schema version.
    pub version: u32,
    /// Managed OS-store aliases; no value hashes or credential contents.
    pub names: BTreeSet<String>,
    /// Deployment paths retained as sensitive even after a binding is removed.
    #[serde(default)]
    pub protected_files: BTreeSet<std::path::PathBuf>,
    /// Exact per-project destination grants.
    pub policy: SecretPolicy,
}

impl Default for Registry {
    /// Creates an empty deny-by-default registry.
    ///
    /// An empty version-one registry.
    /// # Returns
    /// The operation result.
    fn default() -> Self {
        Self {
            version: 1,
            names: BTreeSet::new(),
            protected_files: BTreeSet::new(),
            policy: SecretPolicy::default(),
        }
    }
}
