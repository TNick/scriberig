//! Composes the integrated Scriberig executable from reusable and product runtime crates.

/// Start the integrated `srig` executable and report runtime failures.
///
/// # Returns
/// Unit after the selected command completes.
///
/// # Errors
/// Returns an error when the API server cannot start or serve a request.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match scriberig_cli::parse() {
        scriberig_cli::Command::Serve(args) => {
            scriberig_server::serve(&args.host, args.port, &args.db)?;
        }
        scriberig_cli::Command::Tui => {
            println!("srig tui: terminal UI mode is not implemented yet.");
        }
        scriberig_cli::Command::Run(args) => {
            println!(
                "srig run: one-shot mode is not implemented yet (accepted {} prompt bytes).",
                args.prompt.len()
            );
        }
        scriberig_cli::Command::Agent(args) => {
            println!(
                "srig agent: agent mode is not implemented yet (accepted {} prompt bytes).",
                args.prompt.len()
            );
        }
        scriberig_cli::Command::Maintenance => {
            println!("srig maintenance: maintenance mode is not implemented yet.");
        }
    }
    Ok(())
}
