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
    /// Fetches only dependencies fixed by the exact candidate lockfile.
    CargoDependencyResolution,
    /// Reopens staged XLSX candidates and checks retained-package edit capability.
    XlsxPackage,
    /// Reopens staged DOCX candidates and validates the main document package.
    DocxPackage,
    /// Reopens staged DXF candidates and requires a complete supported drawing.
    DxfPackage,
    /// Reopens staged `GeoPackages` and requires one schema-valid feature change.
    GeoPackagePackage,
    /// Reopens staged QGS/QGZ packages and requires exactly one layer rename.
    QgisPackage,
}

impl CandidateValidationProfile {
    /// Resolves a registered policy identifier.
    ///
    /// # Parameters
    /// - `identifier`: Stable profile identifier supplied by an application caller.
    ///
    /// A registered profile, or `None` when the identifier is unknown.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn from_identifier(identifier: &str) -> Option<Self> {
        match identifier {
            "php-domain-focused" => Some(Self::PhpDomainFocused),
            "project-import-focused" => Some(Self::ProjectImportFocused),
            "rust-workspace" => Some(Self::RustWorkspace),
            "project-ingestion-e2e" => Some(Self::ProjectIngestionE2e),
            "cargo-dependency-resolution" => Some(Self::CargoDependencyResolution),
            "xlsx-package" => Some(Self::XlsxPackage),
            "docx-package" => Some(Self::DocxPackage),
            "dxf-package" => Some(Self::DxfPackage),
            "geopackage-package" => Some(Self::GeoPackagePackage),
            "qgis-package" => Some(Self::QgisPackage),
            _ => None,
        }
    }

    /// Returns the stable provider-facing identity for this profile.
    ///
    /// Registered validation profile identifier.
    #[must_use]
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::PhpDomainFocused => "php-domain-focused",
            Self::ProjectImportFocused => "project-import-focused",
            Self::RustWorkspace => "rust-workspace",
            Self::ProjectIngestionE2e => "project-ingestion-e2e",
            Self::CargoDependencyResolution => "cargo-dependency-resolution",
            Self::XlsxPackage => "xlsx-package",
            Self::DocxPackage => "docx-package",
            Self::DxfPackage => "dxf-package",
            Self::GeoPackagePackage => "geopackage-package",
            Self::QgisPackage => "qgis-package",
        }
    }

    /// Returns fixed Cargo arguments owned by the profile policy.
    ///
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
            Self::CargoDependencyResolution => &["fetch", "--locked"],
            Self::XlsxPackage
            | Self::DocxPackage
            | Self::DxfPackage
            | Self::GeoPackagePackage
            | Self::QgisPackage => &[],
        }
    }

    /// Returns whether this profile may access declared dependency sources.
    ///
    /// `true` only for the bounded, lockfile-based dependency acquisition profile.
    pub(super) const fn allows_dependency_acquisition(self) -> bool {
        matches!(self, Self::CargoDependencyResolution)
    }

    /// Reports whether validation is the bounded retained-package XLSX check.
    pub(super) const fn is_xlsx_package(self) -> bool {
        matches!(self, Self::XlsxPackage)
    }

    /// Reports whether validation reopens staged DOCX packages.
    pub(super) const fn is_docx_package(self) -> bool {
        matches!(self, Self::DocxPackage)
    }

    /// Reports whether validation reopens staged DXF drawings.
    pub(super) const fn is_dxf_package(self) -> bool {
        matches!(self, Self::DxfPackage)
    }

    /// Reports whether validation enforces `GeoPackage` feature-edit semantics.
    pub(super) const fn is_geopackage_package(self) -> bool {
        matches!(self, Self::GeoPackagePackage)
    }

    /// Reports whether validation enforces a single safe QGIS layer rename.
    pub(super) const fn is_qgis_package(self) -> bool {
        matches!(self, Self::QgisPackage)
    }

    /// Returns the maximum process duration for this profile.
    ///
    /// A bounded profile-specific execution deadline.
    pub(super) const fn timeout(self) -> std::time::Duration {
        match self {
            Self::CargoDependencyResolution => std::time::Duration::from_secs(300),
            Self::XlsxPackage
            | Self::DocxPackage
            | Self::DxfPackage
            | Self::GeoPackagePackage
            | Self::QgisPackage => std::time::Duration::from_secs(30),
            Self::PhpDomainFocused
            | Self::ProjectImportFocused
            | Self::RustWorkspace
            | Self::ProjectIngestionE2e => std::time::Duration::from_secs(600),
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
            CandidateValidationProfile::XlsxPackage,
            CandidateValidationProfile::DocxPackage,
            CandidateValidationProfile::DxfPackage,
            CandidateValidationProfile::GeoPackagePackage,
            CandidateValidationProfile::QgisPackage,
            CandidateValidationProfile::CargoDependencyResolution,
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
        let acquisition = CandidateValidationProfile::CargoDependencyResolution;
        assert_eq!(acquisition.cargo_arguments(), &["fetch", "--locked"]);
        assert!(acquisition.allows_dependency_acquisition());
        assert_eq!(acquisition.timeout().as_secs(), 300);
        assert!(
            profiles[..profiles.len() - 1]
                .iter()
                .all(|profile| !profile.allows_dependency_acquisition())
        );
    }
}
