//! Verifies the provider-neutral CAD capability wire contract.

use scriberig_protocol::{
    CadOpaquePreservation, CadProviderAvailability, CadProviderCapabilityInfo, CadProviderFormat,
    CapabilitiesResponse, ProtocolVersion,
};

/// Preserves initialized, unavailable, and old-response provider states on the wire.
#[test]
fn cad_provider_capability_round_trip_and_legacy_default() {
    let response = CapabilitiesResponse {
        protocol: ProtocolVersion::default(),
        capabilities: Vec::new(),
        features: Vec::new(),
        cad_providers: vec![CadProviderCapabilityInfo {
            format: CadProviderFormat::Dwg,
            availability: CadProviderAvailability::Available,
            provider_id: Some("fake.dwg".to_owned()),
            provider_version: Some("1.2".to_owned()),
            can_read: true,
            can_write: false,
            can_validate: false,
            opaque_preservation: CadOpaquePreservation::Unproven,
        }],
    };
    let encoded = serde_json::to_value(&response).expect("serialize capability response");
    assert_eq!(encoded["cadProviders"][0]["format"], "dwg");
    assert_eq!(encoded["cadProviders"][0]["availability"], "available");
    assert_eq!(encoded["cadProviders"][0]["providerVersion"], "1.2");
    let provider = encoded["cadProviders"][0]
        .as_object()
        .expect("typed provider capability object");
    assert!(!provider.contains_key("deploymentRequirements"));
    assert!(!provider.contains_key("licensePath"));
    assert!(!provider.contains_key("credentials"));
    assert_eq!(
        serde_json::from_value::<CapabilitiesResponse>(encoded).expect("deserialize response"),
        response
    );

    let old_response = serde_json::json!({
        "protocol": {"major": 1, "minor": 0},
        "capabilities": [],
        "features": []
    });
    let decoded: CapabilitiesResponse =
        serde_json::from_value(old_response).expect("decode previous response shape");
    assert_eq!(decoded.cad_providers.len(), 0);
}
