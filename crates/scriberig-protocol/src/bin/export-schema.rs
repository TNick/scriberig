//! Exports the authoritative protocol types as a deterministic schema bundle.

use schemars::schema_for;
use scriberig_protocol::{
    CapabilitiesResponse, CommandRequest, Event, InspectEntityRequest, InspectEntityResponse,
    Operation, ProtocolError, QueryRequest, Request, RequestEnvelope, Response, ResponseEnvelope,
    VersionExchange,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Writes all public protocol schemas to the requested output directory.
///
/// An empty result after every schema file is written.
///
/// Returns an error when the output directory or schema cannot be written.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from("target/scriberig-protocol-schema");
    fs::create_dir_all(&output)?;
    write_schema::<CapabilitiesResponse>(&output, "capabilities")?;
    write_schema::<Event>(&output, "event")?;
    write_schema::<InspectEntityRequest>(&output, "inspect-request")?;
    write_schema::<InspectEntityResponse>(&output, "inspect-response")?;
    write_schema::<Operation>(&output, "operation")?;
    write_schema::<ProtocolError>(&output, "error")?;
    write_schema::<CommandRequest>(&output, "command-request")?;
    write_schema::<QueryRequest>(&output, "query-request")?;
    write_schema::<Request>(&output, "request")?;
    write_schema::<Response>(&output, "response")?;
    write_schema::<RequestEnvelope>(&output, "request-envelope")?;
    write_schema::<ResponseEnvelope>(&output, "response-envelope")?;
    write_schema::<VersionExchange>(&output, "version")?;
    Ok(())
}

/// Serializes one schema with stable pretty JSON formatting.
///
/// - `output`: Destination directory.
/// - `name`: Stable schema file stem.
///
/// An empty result after the schema is written.
///
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
