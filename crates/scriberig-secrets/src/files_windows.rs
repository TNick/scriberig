//! Uses safe Windows ACL wrappers to validate opened file handles.

use crate::SecretError;
use std::fs::{File, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use windows_permissions::constants::{AceType, SeObjectType, SecurityInformation};
use windows_permissions::{LocalBox, SecurityDescriptor, Sid};

/// Configures a no-follow open with permission-inspection access.
///
/// - `options`: Open options for a protected file.
pub(super) fn configure(options: &mut OpenOptions) {
    options.custom_flags(0x0020_0000).access_mode(0x8002_0000);
}

/// Requests permission to protect a newly created registry file.
///
/// - `options`: New private file configuration.
pub(super) fn configure_creation(options: &mut OpenOptions) {
    options.custom_flags(0x0020_0000).access_mode(0xc006_0000);
}

/// Returns the current process user's SID using a fixed OS utility.
///
/// The current logon SID as a validated SDDL identifier.
///
/// Returns `Unavailable` when identity cannot be determined.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn current_sid() -> Result<String, SecretError> {
    static SID: std::sync::OnceLock<Result<String, SecretError>> = std::sync::OnceLock::new();
    SID.get_or_init(|| {
        let root = std::env::var_os("SystemRoot").ok_or(SecretError::Unavailable)?;
        let mut command = std::process::Command::new(Path::new(&root).join("System32/whoami.exe"));
        crate::process::prepare_command(&mut command);
        let output = command
            .args(["/user", "/fo", "csv", "/nh"])
            .output()
            .map_err(|_| SecretError::Unavailable)?;
        if !output.status.success() {
            return Err(SecretError::Unavailable);
        }
        let text = std::str::from_utf8(&output.stdout).map_err(|_| SecretError::Unavailable)?;
        let sid = text
            .trim()
            .rsplit(',')
            .next()
            .ok_or(SecretError::Unavailable)?
            .trim_matches('"');
        sid.parse::<LocalBox<Sid>>()
            .map_err(|_| SecretError::Unavailable)?;
        Ok(sid.to_owned())
    })
    .clone()
}

/// Validates that an opened handle grants access only to owner/admin identities.
///
/// - `file`: Open protected file or directory handle.
///
/// Unit for restrictive owner, SYSTEM, and Administrators ACLs.
///
/// Returns `Denied` for broad, missing, or unrecognized ACLs.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn validate(file: &File) -> Result<(), SecretError> {
    let descriptor = windows_permissions::wrappers::GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )
    .map_err(|_| SecretError::Denied)?;
    let user: LocalBox<Sid> = current_sid()?.parse().map_err(|_| SecretError::Denied)?;
    let system: LocalBox<Sid> = "SY".parse().map_err(|_| SecretError::Denied)?;
    let admins: LocalBox<Sid> = "BA".parse().map_err(|_| SecretError::Denied)?;
    let allowed = [&*user, &*system, &*admins];
    if !descriptor
        .owner()
        .is_some_and(|owner| allowed.contains(&owner))
    {
        return Err(SecretError::Denied);
    }
    let acl = descriptor.dacl().ok_or(SecretError::Denied)?;
    for index in 0..acl.len() {
        let ace = acl.get_ace(index).ok_or(SecretError::Denied)?;
        match ace.ace_type() {
            AceType::ACCESS_DENIED_ACE_TYPE => {}
            AceType::ACCESS_ALLOWED_ACE_TYPE
                if ace.sid().is_some_and(|sid| allowed.contains(&sid)) => {}
            _ => return Err(SecretError::Denied),
        }
    }
    Ok(())
}

/// Installs a protected owner/SYSTEM/Administrators DACL on a new file.
///
/// - `file`: New file opened with `WRITE_DAC` permission.
///
/// Unit after installing the protected ACL.
///
/// Returns `Denied` if permissions cannot be restricted.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn protect_file(file: &mut File) -> Result<(), SecretError> {
    let sid = current_sid()?;
    let descriptor: LocalBox<SecurityDescriptor> =
        format!("D:P(A;OICI;FA;;;{sid})(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)")
            .parse()
            .map_err(|_| SecretError::Denied)?;
    windows_permissions::wrappers::SetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        None,
        None,
        descriptor.dacl(),
        None,
    )
    .map_err(|_| SecretError::Denied)
}

/// Protects and validates a registry directory using its actual handle.
///
/// - `path`: New or existing application-owned directory.
///
/// Unit after ACL protection and validation.
///
/// Returns `Denied` on unsafe owners or permissions.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn protect_directory(path: &Path) -> Result<(), SecretError> {
    let mut file = OpenOptions::new()
        .access_mode(0x0006_0080)
        .custom_flags(0x0220_0000)
        .open(path)
        .map_err(|_| SecretError::Denied)?;
    protect_file(&mut file)?;
    validate(&file)
}

/// Requires system policy ownership and mutation grants to be administrative.
///
/// - `file`: Open system policy file.
///
/// Unit for a system/Administrators-owned policy with no ordinary-user writes.
///
/// Returns `Denied` for unsafe ACLs or owners.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn validate_system(file: &File) -> Result<(), SecretError> {
    let descriptor = windows_permissions::wrappers::GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )
    .map_err(|_| SecretError::Denied)?;
    let system: LocalBox<Sid> = "SY".parse().map_err(|_| SecretError::Denied)?;
    let admins: LocalBox<Sid> = "BA".parse().map_err(|_| SecretError::Denied)?;
    let administrators = [&*system, &*admins];
    if !descriptor
        .owner()
        .is_some_and(|owner| administrators.contains(&owner))
    {
        return Err(SecretError::Denied);
    }
    let acl = descriptor.dacl().ok_or(SecretError::Denied)?;
    for index in 0..acl.len() {
        let ace = acl.get_ace(index).ok_or(SecretError::Denied)?;
        match ace.ace_type() {
            AceType::ACCESS_DENIED_ACE_TYPE => {}
            AceType::ACCESS_ALLOWED_ACE_TYPE => {
                if ace.mask().bits() & 0x500d_0116 != 0
                    && !ace.sid().is_some_and(|sid| administrators.contains(&sid))
                {
                    return Err(SecretError::Denied);
                }
            }
            _ => return Err(SecretError::Denied),
        }
    }
    Ok(())
}
