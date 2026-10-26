//! Embeds the exact upstream model-runtime revision locked for this build.

use std::{env, fs, path::PathBuf};

/// Finds the pinned Git revision for `rpi-model-runtime` in the workspace lockfile.
///
/// - `lockfile`: Complete Cargo lockfile text.
///
/// The 40-character Git revision when the package is a Git dependency.
/// # Returns
/// `None` when the package entry or a valid revision is absent.
fn model_runtime_revision(lockfile: &str) -> Option<&str> {
    let mut in_model_runtime = false;
    for line in lockfile.lines() {
        if line == "[[package]]" {
            in_model_runtime = false;
        } else if line == "name = \"rpi-model-runtime\"" {
            in_model_runtime = true;
        } else if in_model_runtime && line.starts_with("source = \"git+") {
            let source = line.strip_prefix("source = \"git+")?.strip_suffix('"')?;
            let (_, revision) = source.rsplit_once('#')?;
            if revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Some(revision);
            }
            return None;
        }
    }
    None
}

/// Writes the lockfile's upstream runtime revision into the crate build output.
///
/// # Errors
/// Returns a filesystem error or panics if the locked upstream revision is absent.
///
/// # Returns
/// Success after writing the pinned revision to Cargo's generated output.
fn main() -> std::io::Result<()> {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let lockfile = manifest.join("../../Cargo.lock");
    println!("cargo:rerun-if-changed={}", lockfile.display());
    let contents = fs::read_to_string(&lockfile)?;
    let revision = model_runtime_revision(&contents)
        .expect("Cargo.lock must pin rpi-model-runtime to an exact Git revision");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("build output directory"));
    fs::write(
        output.join("upstream_model_runtime_revision.rs"),
        format!(
            "/// Exact `rpi-model-runtime` Git revision locked for this product build.\npub const UPSTREAM_MODEL_RUNTIME_REVISION: &str = \"{revision}\";\n"
        ),
    )
}
