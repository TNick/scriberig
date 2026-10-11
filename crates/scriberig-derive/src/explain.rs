//! Provides structured explanations for exact-key reuse and dependency changes.

use crate::dependency::Dependency;
use crate::key::DerivationKey;

/// Identifies why a result can or cannot satisfy a derivation request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExplanationReason {
    /// Stored and requested keys match exactly.
    ExactKeyMatch,
    /// No result record was available.
    NoStoredResult,
    /// Exact derivation profiles differ.
    ProfileMismatch,
    /// An exact entity revision changed.
    RevisionChanged {
        /// Entity whose exact revision differs.
        entity: scriberig_core::EntityId,
    },
    /// A whole-scope snapshot changed.
    SnapshotChanged {
        /// Snapshot declared by the stored result.
        from: scriberig_core::SnapshotId,
        /// Snapshot required by the request.
        to: scriberig_core::SnapshotId,
    },
    /// A completeness/scope dependency changed.
    ScopeChanged {
        /// Opaque completeness scope that changed.
        scope: String,
    },
    /// An exact semantic configuration changed.
    ConfigurationChanged {
        /// Stored exact configuration identity.
        from: String,
        /// Requested exact configuration identity.
        to: String,
    },
    /// A dependency on another result changed.
    DerivedDependencyChanged {
        /// Prior immutable derived-result identity.
        result: scriberig_core::RevisionId,
    },
    /// Observation freshness deadline has passed.
    FreshnessExpired,
    /// Structured result status cannot be reused.
    StatusNotReusable,
    /// Caller policy rejected the otherwise available candidate.
    SelectionRejected,
}

/// Explains a comparison between stored and requested derivation keys.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReuseExplanation {
    /// Machine-readable explanation reason.
    pub reason: ExplanationReason,
    /// Exact stored dependency, if a dependency mismatch caused the result.
    pub stored_dependency: Option<Dependency>,
    /// Exact requested dependency, if a dependency mismatch caused the result.
    pub requested_dependency: Option<Dependency>,
}

/// Compares exact derivation keys and explains whether reuse is justified.
///
/// # Parameters
/// - `stored`: Key attached to the historical record.
/// - `requested`: Key required by the current request.
///
/// # Returns
/// Structured exact-match or first deterministic mismatch explanation.
#[must_use]
pub fn explain_key_comparison(
    stored: &DerivationKey,
    requested: &DerivationKey,
) -> ReuseExplanation {
    if stored == requested {
        return ReuseExplanation {
            reason: ExplanationReason::ExactKeyMatch,
            stored_dependency: None,
            requested_dependency: None,
        };
    }
    if stored.profile != requested.profile {
        return ReuseExplanation {
            reason: ExplanationReason::ProfileMismatch,
            stored_dependency: None,
            requested_dependency: None,
        };
    }
    let left = stored.dependencies.as_slice();
    let right = requested.dependencies.as_slice();
    for old in left {
        if !right.contains(old) {
            let new = right
                .iter()
                .find(|dependency| same_dependency_kind(old, dependency))
                .cloned();
            let reason = match (old, new.as_ref()) {
                (Dependency::ExactRevision { entity, .. }, _) => {
                    ExplanationReason::RevisionChanged { entity: *entity }
                }
                (Dependency::Snapshot(from), Some(Dependency::Snapshot(to))) => {
                    ExplanationReason::SnapshotChanged {
                        from: *from,
                        to: *to,
                    }
                }
                (Dependency::Scope(scope), _) => ExplanationReason::ScopeChanged {
                    scope: scope.clone(),
                },
                (Dependency::Configuration(from), Some(Dependency::Configuration(to))) => {
                    ExplanationReason::ConfigurationChanged {
                        from: from.clone(),
                        to: to.clone(),
                    }
                }
                (Dependency::DerivedResult(result), _) => {
                    ExplanationReason::DerivedDependencyChanged { result: *result }
                }
                (Dependency::Snapshot(from), _) => ExplanationReason::SnapshotChanged {
                    from: *from,
                    to: *from,
                },
                (Dependency::Configuration(from), _) => ExplanationReason::ConfigurationChanged {
                    from: from.clone(),
                    to: String::new(),
                },
            };
            return ReuseExplanation {
                reason,
                stored_dependency: Some(old.clone()),
                requested_dependency: new,
            };
        }
    }
    let requested_dependency = right.iter().find(|new| !left.contains(new)).cloned();
    ReuseExplanation {
        reason: requested_dependency
            .as_ref()
            .map_or(ExplanationReason::SelectionRejected, |new| match new {
                Dependency::ExactRevision { entity, .. } => {
                    ExplanationReason::RevisionChanged { entity: *entity }
                }
                Dependency::Snapshot(to) => {
                    ExplanationReason::SnapshotChanged { from: *to, to: *to }
                }
                Dependency::Scope(scope) => ExplanationReason::ScopeChanged {
                    scope: scope.clone(),
                },
                Dependency::Configuration(to) => ExplanationReason::ConfigurationChanged {
                    from: String::new(),
                    to: to.clone(),
                },
                Dependency::DerivedResult(result) => {
                    ExplanationReason::DerivedDependencyChanged { result: *result }
                }
            }),
        stored_dependency: None,
        requested_dependency,
    }
}

/// Reports whether two dependencies describe the same dependency category.
///
/// # Parameters
/// - `left`: Stored dependency.
/// - `right`: Requested dependency.
///
/// # Returns
/// `true` when one dependency can replace the other by exact identity.
fn same_dependency_kind(left: &Dependency, right: &Dependency) -> bool {
    matches!((left, right),
        (Dependency::ExactRevision { entity: a, .. }, Dependency::ExactRevision { entity: b, .. }) if a == b
    ) || matches!(
        (left, right),
        (Dependency::Snapshot(_), Dependency::Snapshot(_))
    ) || matches!((left, right), (Dependency::Scope(_), Dependency::Scope(_)))
        || matches!(
            (left, right),
            (Dependency::Configuration(_), Dependency::Configuration(_))
        )
        || matches!(
            (left, right),
            (Dependency::DerivedResult(_), Dependency::DerivedResult(_))
        )
}
