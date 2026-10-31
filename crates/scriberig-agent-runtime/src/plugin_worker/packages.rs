//! Resolves explicit extension roots through upstream package metadata.

use std::path::{Path, PathBuf};

/// Reports whether one project contains upstream package registry metadata.
///
/// # Parameters
/// - `project_root`: Host-selected project directory.
///
/// # Returns
/// `true` when the upstream `.rpi/packages.json` registry exists.
pub(super) fn has_project_package_registry(project_root: &Path) -> bool {
    project_root.join(".rpi").join("packages.json").is_file()
}

/// Resolves registered project extension directories beneath authorized roots.
///
/// Project roots are explicit host configuration. The upstream registry reader
/// discovers package resources, while canonical containment checks prevent a
/// registry entry or symlink from escaping the selected project package store.
///
/// # Parameters
/// - `project_roots`: Project roots explicitly selected by the host.
///
/// # Returns
/// Existing canonical extension directories in stable project order.
///
/// # Errors
/// Returns an error for an unreadable root, malformed registry, or resource
/// path that escapes its authorized package store.
pub(super) fn package_extension_dirs(project_roots: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut directories = Vec::new();
    for project_root in project_roots {
        let project_root = project_root
            .canonicalize()
            .map_err(|error| format!("package project root is unavailable: {error}"))?;
        if !project_root.is_dir() {
            return Err("package project root is not a directory".to_owned());
        }
        let package_store = project_root.join(".rpi").join("packages");
        let Some(canonical_store) = canonical_existing_directory(&package_store)? else {
            continue;
        };
        for resource in
            rpi_extensions::packages::project_package_resource_dirs(&project_root, "extensions")?
        {
            let Some(resource) = canonical_existing_directory(&resource)? else {
                continue;
            };
            if !resource.starts_with(&canonical_store) {
                return Err("registered extension directory escapes its package root".to_owned());
            }
            directories.push(resource);
        }
    }
    directories.sort();
    directories.dedup();
    Ok(directories)
}

/// Canonicalizes configured extension directories and rejects non-directories.
///
/// # Parameters
/// - `directories`: Host-configured extension directories.
///
/// # Returns
/// Canonical directories in stable order.
///
/// # Errors
/// Returns an error when a configured directory is missing or invalid.
pub(super) fn canonical_directories(directories: Vec<PathBuf>) -> Result<Vec<PathBuf>, String> {
    let mut canonical = directories
        .into_iter()
        .map(|directory| {
            let directory = directory.canonicalize().map_err(|error| {
                format!("configured extension directory is unavailable: {error}")
            })?;
            if !directory.is_dir() {
                return Err("configured extension path is not a directory".to_owned());
            }
            Ok(directory)
        })
        .collect::<Result<Vec<_>, _>>()?;
    canonical.sort();
    canonical.dedup();
    Ok(canonical)
}

/// Canonicalizes configured plugin files and rejects non-regular files.
///
/// # Parameters
/// - `files`: Host-configured plugin files.
///
/// # Returns
/// Canonical files in stable order.
///
/// # Errors
/// Returns an error when a configured file is missing or not a regular file.
pub(super) fn canonical_files(files: Vec<PathBuf>) -> Result<Vec<PathBuf>, String> {
    let mut canonical = files
        .into_iter()
        .map(|file| {
            let file = file
                .canonicalize()
                .map_err(|error| format!("configured plugin file is unavailable: {error}"))?;
            if !file.is_file() {
                return Err("configured plugin path is not a regular file".to_owned());
            }
            Ok(file)
        })
        .collect::<Result<Vec<_>, _>>()?;
    canonical.sort();
    canonical.dedup();
    Ok(canonical)
}

/// Canonicalizes a directory only when it already exists.
///
/// # Parameters
/// - `directory`: Candidate package resource directory.
///
/// # Returns
/// Its canonical path, or `None` when the directory does not exist.
///
/// # Errors
/// Returns an error when the path exists but cannot be safely inspected.
fn canonical_existing_directory(directory: &Path) -> Result<Option<PathBuf>, String> {
    match std::fs::symlink_metadata(directory) {
        Ok(_) => {
            let canonical = directory
                .canonicalize()
                .map_err(|error| error.to_string())?;
            if !canonical.is_dir() {
                return Err("registered extension resource is not a directory".to_owned());
            }
            Ok(Some(canonical))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    //! Tests upstream registry discovery and host-side canonical authorization.

    use std::path::PathBuf;

    use rpi_extensions::{ProjectPackage, ProjectPackageRegistry};

    use super::package_extension_dirs;

    /// Writes one upstream project package registry for an isolated fixture.
    ///
    /// # Parameters
    /// - `root`: Temporary project root.
    ///
    /// # Returns
    /// The registered package extension directory.
    fn registered_extensions(root: &std::path::Path) -> PathBuf {
        let extension_dir = root
            .join(".rpi/packages/sample/extensions")
            .canonicalize()
            .unwrap_or_else(|_| {
                std::fs::create_dir_all(root.join(".rpi/packages/sample/extensions"))
                    .expect("extension directory");
                root.join(".rpi/packages/sample/extensions")
                    .canonicalize()
                    .expect("canonical extension directory")
            });
        rpi_extensions::packages::write_project_registry(
            root,
            &ProjectPackageRegistry {
                packages: vec![ProjectPackage {
                    name: "sample".to_owned(),
                    version: "1.0.0".to_owned(),
                    source: "local".to_owned(),
                    cargo_package: "sample".to_owned(),
                    artifacts: Vec::new(),
                }],
            },
        )
        .expect("write upstream registry");
        extension_dir
    }

    /// Resolves package resources from the upstream project registry.
    #[test]
    fn discovers_registered_project_extensions() {
        let project = tempfile::tempdir().expect("temporary project");
        let expected = registered_extensions(project.path());

        assert_eq!(
            package_extension_dirs(&[project.path().to_path_buf()]),
            Ok(vec![expected])
        );
    }

    /// Rejects a registered package key that escapes its project package store.
    #[test]
    fn rejects_unsafe_package_name_before_resolving_extension_roots() {
        let project = tempfile::tempdir().expect("temporary project");
        std::fs::create_dir_all(project.path().join(".rpi/packages"))
            .expect("create package store");
        rpi_extensions::packages::write_project_registry(
            project.path(),
            &ProjectPackageRegistry {
                packages: vec![ProjectPackage {
                    name: "../outside".to_owned(),
                    version: "1.0.0".to_owned(),
                    source: "local".to_owned(),
                    cargo_package: "outside".to_owned(),
                    artifacts: Vec::new(),
                }],
            },
        )
        .expect("write registry metadata");

        assert!(package_extension_dirs(&[project.path().to_path_buf()]).is_err());
    }

    /// Refuses an extension resource symlink that escapes the package store.
    #[cfg(unix)]
    #[test]
    fn rejects_project_package_extension_symlink_escape() {
        let project = tempfile::tempdir().expect("temporary project");
        let outside = tempfile::tempdir().expect("outside directory");
        let extension_dir = registered_extensions(project.path());
        std::fs::remove_dir(&extension_dir).expect("remove package extension directory");
        std::os::unix::fs::symlink(outside.path(), &extension_dir)
            .expect("create escaping extension symlink");

        assert!(package_extension_dirs(&[project.path().to_path_buf()]).is_err());
    }
}
