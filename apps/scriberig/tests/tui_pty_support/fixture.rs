//! Provides temporary fixture helpers for native TUI tests.

use std::{
    fs,
    io::Read,
    path::PathBuf,
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
