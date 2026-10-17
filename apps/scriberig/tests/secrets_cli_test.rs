//! Exercises credential inspection through the executable without store access.

use std::process::Command;

/// Inspects references without reading the referenced environment credential.
#[test]
fn inspection_does_not_disclose_environment_values() {
    let directory = tempfile::tempdir().expect("temporary project");
    let output = Command::new(env!("CARGO_BIN_EXE_srig"))
        .current_dir(directory.path())
        .args(["config", "show", "--format", "json"])
        .env(
            "SRIG_CREDENTIAL",
            r#"{"source":"environment","name":"SRIG_TEST_SECRET"}"#,
        )
        .env("SRIG_TEST_SECRET", "CANARY_NEVER_DISPLAY")
        .env_remove("SRIG_API_KEY_ENV")
        .output()
        .expect("config inspection");
    assert!(output.status.success(), "{output:?}");
    let text = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(text.contains("SRIG_TEST_SECRET"));
    assert!(text.contains("SRIG_CREDENTIAL"));
    assert!(!text.contains("CANARY_NEVER_DISPLAY"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("CANARY_NEVER_DISPLAY"));
}

/// Invalid stdin values produce fixed diagnostics and never echo input.
#[test]
fn secret_input_validation_is_safe() {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(env!("CARGO_BIN_EXE_srig"))
        .args(["secrets", "set", "invalid-test", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("secret command");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(b"CANARY\nSECOND_LINE\n")
        .expect("input");
    let output = child.wait_with_output().expect("validation result");
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("CANARY"));
    assert_eq!(output.stdout, [] as [u8; 0]);
}
