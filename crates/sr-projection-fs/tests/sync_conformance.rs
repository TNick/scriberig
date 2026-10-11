//! Verifies exact-byte filesystem projection behavior.

use sr_projection_fs::{
    FileSystem, LocalFileSystem, ProjectionLocation, ProjectionOwnership, digest_bytes,
};
use std::fs;

/// Verifies atomic replacement and exact digest changes.
#[test]
fn atomic_write_preserves_exact_bytes() {
    let root = std::env::temp_dir().join(format!("scriberig-fs-{}", std::process::id()));
    let _ = fs::create_dir_all(&root);
    let path = root.join("projection.txt");
    let io = LocalFileSystem;
    io.write_atomic(&path, b"one\n").expect("write");
    assert_eq!(io.read(&path).expect("read"), b"one\n");
    let first = digest_bytes(b"one\n");
    io.write_atomic(&path, b"one").expect("replace");
    assert_eq!(io.read(&path).expect("read"), b"one");
    assert_ne!(first, digest_bytes(b"one"));
    let _ = fs::remove_dir_all(root);
}

/// Verifies ownership and location remain independent from semantic identity.
#[test]
fn ownership_denies_unmanaged_writes() {
    assert!(!ProjectionOwnership::Unmanaged.may_write());
    assert!(ProjectionOwnership::Unmanaged.forbids_automatic_write());
    assert_ne!(
        ProjectionLocation::new("a.txt"),
        ProjectionLocation::new("b.txt")
    );
}
