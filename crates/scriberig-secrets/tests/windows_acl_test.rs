//! Verifies Windows directory handles support protected registry ACLs.

#![cfg(windows)]

use std::os::windows::fs::OpenOptionsExt;
use windows_permissions::constants::{SeObjectType, SecurityInformation};
use windows_permissions::{LocalBox, SecurityDescriptor};

/// Verifies no-follow directory opens and protected DACL installation.
#[test]
fn directory_handle_supports_registry_protection() {
    let temporary = tempfile::tempdir().expect("temporary path");
    let directory = temporary
        .path()
        .canonicalize()
        .expect("canonical directory");
    scriberig_secrets::files::reject_links(&directory).expect("no linked components");
    let mut file = std::fs::OpenOptions::new()
        .access_mode(0x0006_0080)
        .custom_flags(0x0220_0000)
        .open(&directory)
        .expect("open directory with READ_CONTROL and WRITE_DAC");
    let descriptor = windows_permissions::wrappers::GetSecurityInfo(
        &file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )
    .expect("read owner and DACL");
    let owner = descriptor.owner().expect("owner SID");
    let private: LocalBox<SecurityDescriptor> =
        format!("D:P(A;OICI;FA;;;{owner})(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)")
            .parse()
            .expect("parse DACL");
    windows_permissions::wrappers::SetSecurityInfo(
        &mut file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        None,
        None,
        private.dacl(),
        None,
    )
    .expect("install protected DACL");
}
