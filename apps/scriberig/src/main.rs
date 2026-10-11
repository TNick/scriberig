//! Composes the integrated Scriberig executable from reusable and product runtime crates.

// Block: Delegate process-level command parsing and stub dispatch to the reusable CLI crate.
/// Start the integrated `srig` executable.
fn main() {
    scriberig_cli::run();
}
