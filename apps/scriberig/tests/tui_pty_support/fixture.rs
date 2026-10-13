//! Provides temporary fixture helpers for native TUI tests.

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
};

/// Creates a unique temporary directory owned by one test session.
///
/// # Returns
/// The newly created fixture directory.
///
/// # Parameters
/// - `super`: Parent PTY support module visibility marker.
pub(super) fn temporary_fixture() -> PathBuf {
    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "scriberig-tui-pty-{}-{}",
        std::process::id(),
        sequence
    ));
    fs::create_dir_all(&path).expect("create fixture");
    path
}

/// Creates the generic fixture used by legacy PTY tests.
///
/// # Returns
/// The prepared fixture directory.
///
/// # Parameters
/// - `super`: Parent PTY support module visibility marker.
pub(super) fn basic_fixture() -> PathBuf {
    let fixture = temporary_fixture();
    fs::write(fixture.join("README.md"), "# PTY fixture\n").expect("fixture");
    fs::create_dir_all(fixture.join("src")).expect("source directory");
    fs::write(
        fixture.join("src/app.py"),
        "def greet():\n    return 'hi'\n",
    )
    .expect("python fixture");
    fixture
}

/// Creates and commits a small project for the real bootstrap workflow test.
///
/// # Parameters
/// - `fixture`: Temporary owner directory for the Git checkout.
/// - `super`: Parent PTY support module visibility marker.
///
/// # Returns
/// The committed checkout directory used as the TUI process working directory.
///
/// # Panics
/// Panics when fixture files or Git operations cannot be created.
pub(super) fn bootstrap_project(fixture: &Path) -> PathBuf {
    let checkout = fixture.join("scriberig");
    fs::create_dir_all(checkout.join("src")).expect("create bootstrap source");
    fs::write(
        checkout.join("Cargo.toml"),
        "[package]\nname='scriberig-fixture'\n",
    )
    .expect("write bootstrap package");
    fs::write(
        checkout.join("src/lib.rs"),
        "pub fn greeting() -> &'static str { \"hello\" }\n",
    )
    .expect("write bootstrap source file");
    fs::write(
        checkout.join("AGENTS.md"),
        "# Fixture guidance\nUse semantic source references at the exact project snapshot.\n",
    )
    .expect("write semantic guidance fixture");
    let git = |arguments: &[&str]| {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(&checkout)
                .args(arguments)
                .status()
                .expect("git executable")
                .success()
        );
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    git(&[
        "-c",
        "user.name=Scriberig Test",
        "-c",
        "user.email=test@example.invalid",
        "commit",
        "-qm",
        "bootstrap fixture",
    ]);
    checkout
}

/// Recursively copies a golden project into a temporary test directory.
///
/// # Parameters
/// - `source`: Source fixture directory.
/// - `destination`: Temporary destination directory.
/// - `super`: Parent PTY support module visibility marker.
pub(super) fn copy_directory(source: &PathBuf, destination: &PathBuf) {
    fs::create_dir_all(destination).expect("create golden fixture");
    for entry in fs::read_dir(source).expect("read golden fixture") {
        let entry = entry.expect("read golden entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("copy golden entry");
        }
    }
}

/// Copies a source checkout while excluding generated and local agent data.
///
/// # Parameters
/// - `source`: Repository checkout to copy.
/// - `destination`: Temporary isolated checkout destination.
/// - `super`: Parent PTY support module visibility marker.
///
/// # Returns
/// Unit after all source files and directories have been copied.
///
/// # Panics
/// Panics when a required file cannot be read or copied.
#[allow(dead_code)]
pub(super) fn copy_repository_checkout(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create repository checkout");
    for entry in fs::read_dir(source).expect("read repository source") {
        let entry = entry.expect("read repository entry");
        if matches!(
            entry.file_name().to_str(),
            Some(".git" | ".serena" | "target" | "node_modules")
        ) {
            continue;
        }
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry
            .file_type()
            .expect("inspect repository entry")
            .is_dir()
        {
            copy_repository_checkout(&source_path, &destination_path);
        } else if entry
            .file_type()
            .expect("inspect repository entry")
            .is_file()
        {
            fs::copy(source_path, destination_path).expect("copy repository source file");
        }
    }
}

/// Relays bytes from the PTY reader to the test thread.
#[allow(clippy::needless_pass_by_value)]
pub(super) fn read_output<R: Read>(mut reader: R, sender: mpsc::Sender<Vec<u8>>) {
    let mut buffer = [0_u8; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(size) => {
                if sender.send(buffer[..size].to_vec()).is_err() {
                    return;
                }
            }
        }
    }
}
