//! Verifies mounted-file safety, bounded input, and process-environment boundaries.

use scriberig_secrets::{SecretError, SecretRef, files};

/// Mounted input is bounded and removes exactly one final line ending.
#[test]
fn bounded_secret_input_preserves_value_without_accepting_multiline_data() {
    assert_eq!(
        files::read_secret(&b"token\r\n"[..])
            .expect("single line")
            .expose(),
        "token"
    );
    assert!(files::read_secret(&b"token\n\n"[..]).is_err());
    assert!(files::read_secret(vec![b'x'; 65_537].as_slice()).is_err());
    assert!(files::read_secret(&b"\xff"[..]).is_err());
}

/// Only an actual protected regular file may supply a credential.
#[test]
fn reads_private_files_and_rejects_directories() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let root = temporary.path().canonicalize().expect("canonical path");
    let private = root.join("private");
    files::create_private_dir(&private).expect("private directory");
    let path = private.join("token");
    let mut file = files::create_private_file(&path).expect("private file");
    std::io::Write::write_all(&mut file, b"CANARY_MOUNTED\n").expect("fake secret");
    drop(file);
    assert_eq!(
        files::read_secret_file(&path)
            .expect("mounted value")
            .expose(),
        "CANARY_MOUNTED"
    );
    assert_eq!(
        files::read_secret_file(&private).expect_err("directory rejected"),
        SecretError::Denied
    );
    assert_eq!(
        files::read_secret_file(&private.join("missing")).expect_err("missing file"),
        SecretError::Missing
    );
}

/// Ambient or explicitly configured secret variables cannot survive preparation.
#[test]
fn child_environment_removes_credentials_and_reserves_toolchain_names() {
    let mut command = std::process::Command::new("unused-test-command");
    command.env("PROVIDER_CANARY", "DO_NOT_INHERIT");
    scriberig_secrets::process::prepare_command(&mut command);
    assert!(
        !command
            .get_envs()
            .any(|(name, _)| name == "PROVIDER_CANARY")
    );
    assert!(SecretRef::parse("environment:PATH").is_err());
    assert!(SecretRef::parse("environment:PROVIDER_API_KEY").is_ok());
}

/// Known secret artifacts are blocked while explicit example templates remain usable.
#[test]
fn sensitive_artifacts_are_excluded() {
    for path in [
        ".env",
        "nested/.env.production",
        ".ssh/id_ed25519",
        "keys/server.pem",
        ".npmrc",
        ".aws/credentials",
    ] {
        assert!(scriberig_secrets::sensitive::sensitive_name(
            std::path::Path::new(path)
        ));
    }
    assert!(!scriberig_secrets::sensitive::sensitive_name(
        std::path::Path::new(".env.example")
    ));
}
