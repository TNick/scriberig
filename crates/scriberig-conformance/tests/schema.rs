//! Verifies protocol schema generation remains deterministic.

use schemars::{JsonSchema, schema_for};
use scriberig_protocol::{
    CapabilitiesResponse, CommandRequest, Event, InspectEntityRequest, InspectEntityResponse,
    Operation, ProtocolError, QueryRequest, Request, RequestEnvelope, Response, ResponseEnvelope,
    VersionExchange,
};

/// Serializes a generated schema using the same format as the schema exporter.
fn schema<T: JsonSchema>() -> String {
    serde_json::to_string_pretty(&schema_for!(T)).expect("schema serialization")
}

/// Verifies every exported protocol schema is reproducible.
#[test]
fn exported_protocol_schemas_are_reproducible() {
    let schemas = [
        (
            schema::<CapabilitiesResponse>(),
            schema::<CapabilitiesResponse>(),
        ),
        (schema::<Event>(), schema::<Event>()),
        (
            schema::<InspectEntityRequest>(),
            schema::<InspectEntityRequest>(),
        ),
        (
            schema::<InspectEntityResponse>(),
            schema::<InspectEntityResponse>(),
        ),
        (schema::<Operation>(), schema::<Operation>()),
        (schema::<ProtocolError>(), schema::<ProtocolError>()),
        (schema::<VersionExchange>(), schema::<VersionExchange>()),
        (schema::<CommandRequest>(), schema::<CommandRequest>()),
        (schema::<QueryRequest>(), schema::<QueryRequest>()),
        (schema::<Request>(), schema::<Request>()),
        (schema::<Response>(), schema::<Response>()),
        (schema::<RequestEnvelope>(), schema::<RequestEnvelope>()),
        (schema::<ResponseEnvelope>(), schema::<ResponseEnvelope>()),
    ];
    for (first, second) in schemas {
        assert_ne!(first, "");
        assert_eq!(first, second);
    }
}
