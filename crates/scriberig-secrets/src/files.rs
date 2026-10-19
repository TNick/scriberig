//! Opens bounded protected files without following symbolic links.

use crate::{SecretError, SecretValue};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::Path;
use zeroize::Zeroizing;

#[cfg(windows)]
#[path = "files_windows.rs"]
mod platform;
#[cfg(unix)]
#[path = "files_unix.rs"]
mod platform;

/// Rejects symbolic links and Windows reparse points in every existing component.
///
/// - `path`: Absolute protected path to inspect.
///
/// Unit for an existing path without link components.
///
/// Returns a safe denial for missing, relative, or redirected paths.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn reject_links(path: &Path) -> Result<(), SecretError> {
    if !path.is_absolute() {
        return Err(SecretError::Invalid);
    }
    let mut current = std::path::PathBuf::new();
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(SecretError::Denied);
        }
        current.push(component);
        if matches!(component, std::path::Component::Prefix(_)) {
            continue;
        }
        let metadata = std::fs::symlink_metadata(&current).map_err(|_| SecretError::Denied)?;
        if metadata.is_symlink() {
            return Err(SecretError::Denied);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(SecretError::Denied);
            }
        }
    }
    Ok(())
}

/// Opens a regular protected file and validates its actual handle permissions.
///
/// - `path`: Absolute existing file path.
///
/// A verified open file handle.
///
/// Returns a safe missing, permission, or path validation failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn open_private(path: &Path) -> Result<File, SecretError> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(SecretError::Missing);
        }
        Err(_) => return Err(SecretError::Denied),
        Ok(_) => {}
    }
    reject_links(path)?;
    let mut options = OpenOptions::new();
    options.read(true);
    platform::configure(&mut options);
    let file = options.open(path).map_err(|_| SecretError::Denied)?;
    if !file.metadata().map_err(|_| SecretError::Denied)?.is_file() {
        return Err(SecretError::Denied);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if file
            .metadata()
            .map_err(|_| SecretError::Denied)?
            .file_attributes()
            & 0x400
            != 0
        {
            return Err(SecretError::Denied);
        }
    }
    platform::validate(&file)?;
    Ok(file)
}

/// Reads a bounded mounted credential into a zeroizing value.
///
/// - `path`: Absolute protected file path.
///
/// A protected UTF-8 credential with one terminal line ending removed.
///
/// Returns a safe failure for unsafe files, invalid text, or invalid values.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn read_secret_file(path: &Path) -> Result<SecretValue, SecretError> {
    let file = open_private(path)?;
    read_secret(file)
}

/// Reads secret input without permitting an unbounded allocation.
///
/// - `reader`: Stdin or a protected file reader.
///
/// A protected credential after removing one terminal line ending.
///
/// Returns `Invalid` for oversized, empty, multiline, or invalid UTF-8 input.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn read_secret(reader: impl Read) -> Result<SecretValue, SecretError> {
    let mut bytes = Zeroizing::new(Vec::new());
    reader
        .take(65_539)
        .read_to_end(&mut bytes)
        .map_err(|_| SecretError::Invalid)?;
    let length = bytes.len();
    if bytes.ends_with(b"\r\n") {
        bytes.truncate(length - 2);
    } else if bytes.ends_with(b"\n") {
        bytes.truncate(length - 1);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| SecretError::Invalid)?;
    SecretValue::new(text.to_owned())
}

/// Creates and protects an application-owned registry directory.
///
/// - `path`: Absolute registry directory path.
///
/// Unit once the directory has restrictive permissions.
///
/// Returns a safe permission or filesystem failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn create_private_dir(path: &Path) -> Result<(), SecretError> {
    if !path.exists() {
        let parent = path.parent().ok_or(SecretError::Invalid)?;
        std::fs::create_dir_all(parent).map_err(|_| SecretError::Denied)?;
        reject_links(parent)?;
        std::fs::create_dir(path).map_err(|_| SecretError::Denied)?;
    }
    reject_links(path)?;
    platform::protect_directory(path)
}

/// Protects a newly created registry file before writing its contents.
///
/// - `file`: Application-created file handle.
///
/// Unit after restrictive permissions are installed and checked.
///
/// Returns a safe permission failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn protect_new_file(file: &mut File) -> Result<(), SecretError> {
    platform::protect_file(file)?;
    platform::validate(file)
}

/// Creates a new protected file without overwriting an existing path.
///
/// - `path`: Absolute new file path inside a protected directory.
///
/// A readable and writable private file handle.
///
/// Returns a safe failure on collisions, unsafe parents, or permission errors.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn create_private_file(path: &Path) -> Result<File, SecretError> {
    reject_links(path.parent().ok_or(SecretError::Invalid)?)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    platform::configure_creation(&mut options);
    let mut file = options.open(path).map_err(|_| SecretError::Denied)?;
    protect_new_file(&mut file)?;
    Ok(file)
}

/// Opens an administrator-owned policy that ordinary users cannot modify.
///
/// - `path`: Absolute fixed system policy path.
///
/// A regular no-follow file with validated administrator write authority.
///
/// Returns `Denied` for unsafe owners, links, or permission grants.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn open_system_policy(path: &Path) -> Result<File, SecretError> {
    reject_links(path)?;
    let mut options = OpenOptions::new();
    options.read(true);
    platform::configure(&mut options);
    let file = options.open(path).map_err(|_| SecretError::Denied)?;
    if !file.metadata().map_err(|_| SecretError::Denied)?.is_file() {
        return Err(SecretError::Denied);
    }
    platform::validate_system(&file)?;
    Ok(file)
}
