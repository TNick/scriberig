//! Exports the authoritative protocol types as a deterministic schema bundle.

use schemars::schema_for;
use scriberig_protocol::{
    CapabilitiesResponse, Event, InspectEntityRequest, InspectEntityResponse, Operation,
    ProtocolError, VersionExchange,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Writes all public protocol schemas to the requested output directory.
///
/// # Returns
/// An empty result after every schema file is written.
///
/// # Errors
/// Returns an error when the output directory or schema cannot be written.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from("target/scriberig-protocol-schema");
    fs::create_dir_all(&output)?;
    write_schema::<CapabilitiesResponse>(&output, "capabilities")?;
    write_schema::<Event>(&output, "event")?;
    write_schema::<InspectEntityRequest>(&output, "inspect-request")?;
    write_schema::<InspectEntityResponse>(&output, "inspect-response")?;
    write_schema::<Operation>(&output, "operation")?;
    write_schema::<ProtocolError>(&output, "error")?;
    write_schema::<VersionExchange>(&output, "version")?;
    Ok(())
}

/// Serializes one schema with stable pretty JSON formatting.
///
/// # Parameters
/// - `output`: Destination directory.
/// - `name`: Stable schema file stem.
///
/// # Returns
/// An empty result after the schema is written.
///
/// # Errors
/// Returns an error when serialization or file writing fails.
fn write_schema<T: schemars::JsonSchema>(
    output: &Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let schema = schema_for!(T);
    let path = output.join(format!("{name}.json"));
    fs::write(path, serde_json::to_vec_pretty(&schema)?)?;
    Ok(())
}
