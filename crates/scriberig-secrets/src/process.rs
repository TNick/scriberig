//! Supplies a minimal inherited environment for project subprocesses.

use std::process::Command;

/// Toolchain variables that cannot also be configured credential references.
const ALLOWED: &[&str] = &[
    "PATH",
    "SystemRoot",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
    "TEMP",
    "TMP",
    "HOME",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "LANG",
    "LC_ALL",
    "TERM",
    "INCLUDE",
    "LIB",
    "LIBPATH",
    "WindowsSdkDir",
    "VCToolsInstallDir",
];

/// Clears ambient credentials and retains documented toolchain environment.
///
/// # Parameters
/// - `command`: Command to prepare before adding explicit safe variables.
pub fn prepare_command(command: &mut Command) {
    command.env_clear();
    for name in ALLOWED {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
}

/// Detects reserved toolchain names when validating environment references.
///
/// # Parameters
/// - `name`: Candidate credential environment name.
///
/// # Returns
/// Whether the name is reserved for process setup.
pub(crate) fn is_toolchain_variable(name: &str) -> bool {
    ALLOWED
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(name))
}
