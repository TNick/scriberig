//! Reads protected registries and serializes atomic metadata updates.

use super::Registry;
use crate::{SecretError, SecretPolicy, files};
use fs2::FileExt;
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Selects user metadata and optional administrator restrictions.
#[derive(Clone, Debug)]
pub struct RegistryLocations {
    /// Protected user registry directory.
    pub user_dir: PathBuf,
    /// Fixed administrator policy path, when the platform has one.
    pub system_policy: Option<PathBuf>,
}

impl RegistryLocations {
    /// Resolves conventional platform directories without project overrides.
    ///
    /// Registry locations for the current user and platform.
    ///
    /// Returns `Unavailable` when the user directory is unknown.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn from_process() -> Result<Self, SecretError> {
        #[cfg(windows)]
        let (user, system) = (
            std::env::var_os("APPDATA").map(|root| PathBuf::from(root).join("Scriberig")),
            std::env::var_os("PROGRAMDATA").map(|root| PathBuf::from(root).join("Scriberig")),
        );
        #[cfg(target_os = "macos")]
        let (user, system) = (
            std::env::var_os("HOME")
                .map(|root| PathBuf::from(root).join("Library/Application Support/Scriberig")),
            Some(PathBuf::from("/etc/scriberig")),
        );
        #[cfg(all(unix, not(target_os = "macos")))]
        let (user, system) = (
            std::env::var_os("XDG_CONFIG_HOME")
                .filter(|root| !root.is_empty())
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|root| PathBuf::from(root).join(".config"))
                })
                .map(|root| root.join("scriberig")),
            Some(PathBuf::from("/etc/scriberig")),
        );
        let user_dir = user.ok_or(SecretError::Unavailable)?.join("secrets");
        if !user_dir.is_absolute() {
            return Err(SecretError::Invalid);
        }
        Ok(Self {
            user_dir,
            system_policy: system.map(|root| root.join("secrets/policy.json")),
        })
    }

    /// Reads metadata without creating directories or retrieving credentials.
    ///
    /// The user registry, or an empty deny-by-default registry.
    ///
    /// Returns a safe corruption or permission failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn load(&self) -> Result<Registry, SecretError> {
        let registry = read_optional::<Registry>(&self.user_dir.join("registry.json"), false)?
            .unwrap_or_default();
        if registry.version != 1 {
            return Err(SecretError::Invalid);
        }
        Ok(registry)
    }

    /// Loads administrator restrictions if explicitly installed.
    ///
    /// An additional mandatory policy, or `None` when none is installed.
    ///
    /// Returns a safe corruption or permission failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn system(&self) -> Result<Option<SecretPolicy>, SecretError> {
        self.system_policy
            .as_deref()
            .map(|path| read_optional(path, true))
            .transpose()
            .map(Option::flatten)
    }

    /// Serializes local registry mutations using an OS file lock.
    ///
    /// A lock guard holding the current metadata.
    ///
    /// Returns a safe directory, lock, or registry failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn lock(&self) -> Result<RegistryGuard, SecretError> {
        files::create_private_dir(&self.user_dir)?;
        let lock_path = self.user_dir.join("registry.lock");
        let file = match std::fs::symlink_metadata(&lock_path) {
            Ok(_) => files::open_private(&lock_path)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                match files::create_private_file(&lock_path) {
                    Ok(file) => file,
                    Err(_) if lock_path.exists() => files::open_private(&lock_path)?,
                    Err(error) => return Err(error),
                }
            }
            Err(_) => return Err(SecretError::Denied),
        };
        file.lock_exclusive()
            .map_err(|_| SecretError::Unavailable)?;
        Ok(RegistryGuard {
            registry: self.load()?,
            directory: self.user_dir.clone(),
            _lock: file,
        })
    }
}

/// Holds an exclusive registry lock across store and metadata operations.
pub struct RegistryGuard {
    /// Current secret-free metadata.
    pub registry: Registry,
    /// Protected directory containing atomic registry updates.
    directory: PathBuf,
    /// OS lock released when this guard is dropped.
    _lock: File,
}

impl RegistryGuard {
    /// Atomically persists the metadata while holding its exclusive lock.
    ///
    /// Unit once metadata is durably replaced.
    ///
    /// Returns a safe write or replacement failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn save(&self) -> Result<(), SecretError> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let temporary = self
            .directory
            .join(format!("registry-{}-{id}.tmp", std::process::id()));
        let result = (|| {
            let mut file = files::create_private_file(&temporary)?;
            let bytes =
                serde_json::to_vec_pretty(&self.registry).map_err(|_| SecretError::Invalid)?;
            file.write_all(&bytes)
                .map_err(|_| SecretError::Unavailable)?;
            file.sync_all().map_err(|_| SecretError::Unavailable)?;
            drop(file);
            std::fs::rename(&temporary, self.directory.join("registry.json"))
                .map_err(|_| SecretError::Unavailable)?;
            #[cfg(unix)]
            File::open(&self.directory)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| SecretError::Unavailable)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }
}

/// Reads a bounded protected JSON document, distinguishing absence from failure.
///
/// - `path`: Absolute registry or policy file.
/// - `system`: Whether administrator ownership and write restrictions apply.
///
/// A parsed document, or no document when the path is absent.
///
/// Returns a safe permission, size, or document validation failure.
fn read_optional<T: DeserializeOwned>(path: &Path, system: bool) -> Result<Option<T>, SecretError> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(SecretError::Denied),
        Ok(_) => {}
    }
    let file = if system {
        files::open_system_policy(path)?
    } else {
        files::open_private(path)?
    };
    let mut bytes = Vec::new();
    file.take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|_| SecretError::Unavailable)?;
    if bytes.len() > 1_048_576 {
        return Err(SecretError::Invalid);
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| SecretError::Invalid)
}
