//! Applies Unix owner-only permissions to opened protected files.

use crate::SecretError;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

/// Configures no-follow and nonblocking flags for a protected open.
///
/// - `options`: File open configuration.
pub(super) fn configure(options: &mut OpenOptions) {
    let flags = (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits();
    // `custom_flags` expects `i32`; preserve the bitmask instead of numerically casting it.
    options.custom_flags(i32::from_ne_bytes(flags.to_ne_bytes()));
}

/// Restricts initial creation permissions before contents are written.
///
/// - `options`: New private file configuration.
pub(super) fn configure_creation(options: &mut OpenOptions) {
    configure(options);
    options.mode(0o600);
}

/// Validates owner and access mode from the opened handle.
///
/// - `file`: Open file handle.
///
/// Unit for owner-only files owned by the current user or root.
///
/// Returns `Denied` for unsafe permissions or owners.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn validate(file: &File) -> Result<(), SecretError> {
    let metadata = file.metadata().map_err(|_| SecretError::Denied)?;
    let uid = rustix::process::geteuid().as_raw();
    if (metadata.uid() != uid && metadata.uid() != 0) || metadata.mode() & 0o077 != 0 {
        return Err(SecretError::Denied);
    }
    Ok(())
}

/// Installs owner-only access on a new file.
///
/// - `file`: New application-owned file handle.
///
/// Unit after permissions are installed.
///
/// Returns `Denied` on filesystem failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn protect_file(file: &mut File) -> Result<(), SecretError> {
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
        .map_err(|_| SecretError::Denied)
}

/// Installs owner-only directory permissions.
///
/// - `path`: Registry directory.
///
/// Unit after directory permissions are installed.
///
/// Returns `Denied` if the directory is not owned by the current user.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn protect_directory(path: &Path) -> Result<(), SecretError> {
    let file = File::open(path).map_err(|_| SecretError::Denied)?;
    if file.metadata().map_err(|_| SecretError::Denied)?.uid()
        != rustix::process::geteuid().as_raw()
    {
        return Err(SecretError::Denied);
    }
    file.set_permissions(std::fs::Permissions::from_mode(0o700))
        .map_err(|_| SecretError::Denied)
}

/// Requires root ownership and denies nonadministrator write permissions.
///
/// - `file`: Open system policy file.
///
/// Unit when only root can modify the policy.
///
/// Returns `Denied` for unsafe owners or write permissions.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn validate_system(file: &File) -> Result<(), SecretError> {
    let metadata = file.metadata().map_err(|_| SecretError::Denied)?;
    if metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
        return Err(SecretError::Denied);
    }
    Ok(())
}
