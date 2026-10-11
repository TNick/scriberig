//! Defines exact compiler-analysis profile and environment inputs.

use scriberig_core::{EntityId, RevisionId};
use scriberig_derive::{Dependency, DependencySet, DerivationProfile};

/// Exact environment choices that affect Rust analysis semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisProfile {
    /// Rust toolchain or compiler version label.
    pub toolchain: String,
    /// Compilation target triple.
    pub target: String,
    /// Enabled Cargo features in deterministic order.
    pub features: Vec<String>,
    /// Workspace or dependency graph identity.
    pub workspace: String,
    /// Adapter-specific configuration identity.
    pub configuration: String,
}

impl AnalysisProfile {
    /// Creates an exact analysis profile.
    ///
    /// # Parameters
    /// - `toolchain`: Compiler or toolchain version.
    /// - `target`: Compilation target triple.
    /// - `features`: Enabled features.
    /// - `workspace`: Dependency/workspace identity.
    /// - `configuration`: Analyzer configuration identity.
    ///
    /// # Returns
    /// A normalized profile with sorted feature names.
    #[must_use]
    pub fn new(
        toolchain: impl Into<String>,
        target: impl Into<String>,
        mut features: Vec<String>,
        workspace: impl Into<String>,
        configuration: impl Into<String>,
    ) -> Self {
        features.sort();
        features.dedup();
        Self {
            toolchain: toolchain.into(),
            target: target.into(),
            features,
            workspace: workspace.into(),
            configuration: configuration.into(),
        }
    }

    /// Returns the shared derivation profile identity.
    ///
    /// # Returns
    /// A versioned profile containing producer and configuration identities.
    #[must_use]
    pub fn derivation_profile(&self) -> DerivationProfile {
        DerivationProfile::new("rust.compiler-analysis", "v1")
            .with_producer(format!("rustc:{}", self.toolchain))
            .with_configuration(self.configuration_id())
    }

    /// Returns a deterministic configuration identity for this environment.
    ///
    /// # Returns
    /// A stable configuration string.
    #[must_use]
    pub fn configuration_id(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.target,
            self.features.join(","),
            self.workspace,
            self.configuration
        )
    }

    /// Builds exact derivation dependencies for one source revision.
    ///
    /// # Parameters
    /// - `entity`: Canonical source entity.
    /// - `revision`: Canonical Rust syntax revision.
    ///
    /// # Returns
    /// Source and environment dependencies sufficient for invalidation.
    #[must_use]
    pub fn dependencies(&self, entity: EntityId, revision: RevisionId) -> DependencySet {
        DependencySet::new(vec![
            Dependency::ExactRevision { entity, revision },
            Dependency::Configuration(self.configuration_id()),
        ])
    }
}
