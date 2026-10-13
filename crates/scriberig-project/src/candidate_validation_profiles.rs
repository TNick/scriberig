//! Defines registered project candidate validation policies.

/// Selects a bounded validation policy for an exact project-wide candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateValidationProfile {
    /// Tests the proposed PHP domain adapter and its parser contract.
    PhpDomainFocused,
    /// Tests one end-to-end PHP project import and semantic projection.
    ProjectImportFocused,
    /// Checks every Rust workspace target in the candidate source tree.
    RustWorkspace,
    /// Runs the complete PHP project ingestion conformance test target.
    ProjectIngestionE2e,
}

impl CandidateValidationProfile {
    /// Resolves a registered policy identifier.
    ///
    /// # Parameters
    /// - `identifier`: Stable profile identifier supplied by an application caller.
    ///
    /// # Returns
    /// A registered profile, or `None` when the identifier is unknown.
    #[must_use]
    pub fn from_identifier(identifier: &str) -> Option<Self> {
        match identifier {
            "php-domain-focused" => Some(Self::PhpDomainFocused),
            "project-import-focused" => Some(Self::ProjectImportFocused),
            "rust-workspace" => Some(Self::RustWorkspace),
            "project-ingestion-e2e" => Some(Self::ProjectIngestionE2e),
            _ => None,
        }
    }

    /// Returns the stable provider-facing identity for this profile.
    ///
    /// # Returns
    /// Registered validation profile identifier.
    #[must_use]
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::PhpDomainFocused => "php-domain-focused",
            Self::ProjectImportFocused => "project-import-focused",
            Self::RustWorkspace => "rust-workspace",
            Self::ProjectIngestionE2e => "project-ingestion-e2e",
        }
    }

    /// Returns fixed Cargo arguments owned by the profile policy.
    ///
    /// # Returns
    /// Arguments passed directly to Cargo without a shell.
    pub(super) const fn cargo_arguments(self) -> &'static [&'static str] {
        match self {
            Self::PhpDomainFocused => &["test", "--offline", "--package", "scriberig-domain-php"],
            Self::ProjectImportFocused => &[
                "test",
                "--offline",
                "--package",
                "scriberig-project",
                "--test",
                "php_ingestion",
                "imports_php_bytes_and_discovers_source_declarations",
            ],
            Self::RustWorkspace => &["check", "--workspace", "--all-targets", "--offline"],
            Self::ProjectIngestionE2e => &[
                "test",
                "--offline",
                "--package",
                "scriberig-project",
                "--test",
                "php_ingestion",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CandidateValidationProfile;

    /// Verifies registered profile identities are unique and resolve deterministically.
    #[test]
    fn profile_registry_round_trips_all_registered_identifiers() {
        let profiles = [
            CandidateValidationProfile::PhpDomainFocused,
            CandidateValidationProfile::ProjectImportFocused,
            CandidateValidationProfile::RustWorkspace,
            CandidateValidationProfile::ProjectIngestionE2e,
        ];
        for (index, profile) in profiles.iter().enumerate() {
            assert_eq!(
                CandidateValidationProfile::from_identifier(profile.identifier()),
                Some(*profile)
            );
            assert!(
                profiles[..index]
                    .iter()
                    .all(|prior| prior.identifier() != profile.identifier())
            );
        }
        assert_eq!(
            CandidateValidationProfile::from_identifier("arbitrary-shell"),
            None
        );
    }
}
