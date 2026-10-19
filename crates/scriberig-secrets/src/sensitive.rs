//! Excludes known secret artifacts and registered deployment paths from knowledge.

use crate::registry::RegistryLocations;
use crate::{SecretError, SecretRef};
use std::path::{Path, PathBuf};

/// Holds a secret-free snapshot of paths excluded from import and projection.
pub struct SensitivePaths {
    /// Registered protected files and administrative metadata directories.
    paths: Vec<PathBuf>,
}

impl SensitivePaths {
    /// Reads exclusion metadata without retrieving credential values.
    ///
    /// Known secret paths from user and system authorization metadata.
    ///
    /// Returns a safe failure instead of importing when policy cannot be read.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn from_process() -> Result<Self, SecretError> {
        let locations = RegistryLocations::from_process()?;
        let registry = locations.load()?;
        let mut paths = registry.protected_files.into_iter().collect::<Vec<_>>();
        paths.push(locations.user_dir.clone());
        if let Some(system) = locations.system()? {
            paths.extend(system.bindings.into_iter().filter_map(|binding| {
                if let SecretRef::File { path } = binding.credential {
                    Some(path)
                } else {
                    None
                }
            }));
        }
        if let Some(system) = locations.system_policy {
            paths.push(system);
        }
        paths = paths
            .into_iter()
            .map(|path| path.canonicalize().unwrap_or(path))
            .collect();
        Ok(Self { paths })
    }

    /// Checks known names and registered absolute paths without reading contents.
    ///
    /// - `path`: File or directory being considered for import or projection.
    ///
    /// Whether the path must be excluded from semantic knowledge.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn contains(&self, path: &Path) -> bool {
        if sensitive_name(path) {
            return true;
        }
        let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.paths
            .iter()
            .any(|protected| resolved == *protected || resolved.starts_with(protected))
    }
}

/// Recognizes conservative credential filenames while allowing explicit templates.
///
/// - `path`: Candidate path whose components are examined.
///
/// Whether a component identifies a common credential file or directory.
#[must_use]
/// # Returns
/// The operation result.
pub fn sensitive_name(path: &Path) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy().to_ascii_lowercase();
        if matches!(
            name.as_str(),
            ".env.example" | ".env.sample" | ".env.template" | ".env.dist"
        ) {
            return false;
        }
        name == ".env"
            || name.starts_with(".env.")
            || matches!(
                name.as_str(),
                ".ssh"
                    | ".aws"
                    | ".gnupg"
                    | ".kube"
                    | ".docker"
                    | "id_rsa"
                    | "id_dsa"
                    | "id_ecdsa"
                    | "id_ed25519"
                    | ".netrc"
                    | "_netrc"
                    | ".npmrc"
                    | ".pypirc"
                    | ".git-credentials"
            )
            || [".key", ".pem", ".p12", ".pfx"]
                .iter()
                .any(|extension| name.ends_with(extension))
    })
}
