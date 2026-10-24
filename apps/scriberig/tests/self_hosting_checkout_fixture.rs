//! Guards the repository checkout copied by the real-process self-hosting gates.

#[allow(dead_code)]
#[path = "tui_pty_support/fixture.rs"]
mod checkout_fixture;

use std::fs;
use std::path::Path;

/// Verifies the copied self-hosting checkout carries only Git-indexed entries.
///
/// The gates onboard this copy through the real import path, so an untracked
/// scratch file in the developer checkout changes the scanned project surface.
/// With untracked local scripts and logs present, onboarding never imported the
/// project and its agent root ended `Abandoned` with `No more faux responses
/// queued`.
#[test]
fn copied_self_hosting_checkout_excludes_untracked_scratch() {
    let fixture = checkout_fixture::temporary_fixture();
    let source = fixture.join("repository");
    write_indexed_repository(&source);
    fs::create_dir_all(source.join("scratch")).expect("create scratch directory");
    fs::write(
        source.join("scratch/broken.js"),
        "return { unparseable: at the top level };\n",
    )
    .expect("write untracked scratch source");

    let destination = fixture.join("checkout");
    checkout_fixture::copy_repository_checkout(&source, &destination);

    assert!(destination.join("Cargo.toml").is_file(), "tracked manifest");
    assert!(destination.join("src/lib.rs").is_file(), "tracked source");
    assert!(
        !destination.join("scratch/broken.js").exists(),
        "untracked scratch entered the self-hosting checkout"
    );
    fs::remove_dir_all(fixture).expect("remove copy fixture");
}

/// Writes a minimal Git-indexed repository whose entries the copy must carry.
///
/// - `source`: Directory that becomes the Git-indexed copy source.
///
/// # Panics
/// Panics when a fixture file or Git command cannot be prepared.
fn write_indexed_repository(source: &Path) {
    fs::create_dir_all(source.join("src")).expect("create repository source");
    fs::write(
        source.join("Cargo.toml"),
        "[package]\nname='copy-fixture'\n",
    )
    .expect("write manifest");
    fs::write(source.join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n").expect("write source");
    let git = |arguments: &[&str]| {
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(source)
                .args(arguments)
                .status()
                .expect("git executable")
                .success(),
            "Git command failed: {arguments:?}"
        );
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
}
