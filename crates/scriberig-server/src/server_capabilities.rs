//! Advertises protocol capabilities supported by the application façade.

use super::RuntimeCapability;

/// Converts shared CAD provider status to the provider-neutral protocol DTO.
///
/// # Parameters
/// - `status`: Shared application status for one external CAD format.
///
/// # Returns
/// A wire value without native handles, credentials, or provider diagnostic text.
pub(super) fn cad_provider_capability(
    status: scriberig_domain_cad::CadProviderStatus,
) -> scriberig_protocol::CadProviderCapabilityInfo {
    use scriberig_domain_cad::{CadFormat, CadProviderOpaquePreservation, CadProviderStatus};
    use scriberig_protocol::{
        CadOpaquePreservation, CadProviderAvailability, CadProviderCapabilityInfo,
        CadProviderFormat,
    };

    let format = |format: CadFormat| match format {
        CadFormat::Dwg => CadProviderFormat::Dwg,
        CadFormat::Dxf => CadProviderFormat::Dxf,
        CadFormat::Other(value) => CadProviderFormat::Other(value),
    };
    let preservation = |value| match value {
        CadProviderOpaquePreservation::Preserved => CadOpaquePreservation::Preserved,
        CadProviderOpaquePreservation::Unproven => CadOpaquePreservation::Unproven,
        CadProviderOpaquePreservation::NotPreserved => CadOpaquePreservation::NotPreserved,
    };

    match status {
        CadProviderStatus::Available(descriptor) => {
            let capability = &descriptor.formats[0];
            CadProviderCapabilityInfo {
                format: format(capability.format.clone()),
                availability: CadProviderAvailability::Available,
                provider_id: Some(descriptor.id),
                provider_version: Some(descriptor.version),
                can_read: capability.read,
                can_write: capability.write,
                can_validate: capability.validate,
                opaque_preservation: preservation(capability.opaque_preservation),
            }
        }
        CadProviderStatus::Unavailable(unavailable) => CadProviderCapabilityInfo {
            format: format(unavailable.format),
            availability: CadProviderAvailability::Unavailable,
            provider_id: None,
            provider_version: None,
            can_read: false,
            can_write: false,
            can_validate: false,
            opaque_preservation: CadOpaquePreservation::Unproven,
        },
        CadProviderStatus::Failed { format: value, .. } => CadProviderCapabilityInfo {
            format: format(value),
            availability: CadProviderAvailability::InitializationFailed,
            provider_id: None,
            provider_version: None,
            can_read: false,
            can_write: false,
            can_validate: false,
            opaque_preservation: CadOpaquePreservation::Unproven,
        },
    }
}

/// Builds the static capability list supported by this server façade.
///
/// - `super`: Parent module containing the protocol application adapter.
///
/// Available inspect, explorer, and agent session capabilities.
/// # Returns
/// The operation result.
pub(super) fn capabilities() -> Vec<RuntimeCapability> {
    [
        ("inspect_entity", "Inspect one entity"),
        ("explorer_list", "List entities in an exact snapshot"),
        ("agent_start", "Start provider-neutral agent work"),
        ("agent_continue", "Continue work on a session branch"),
        ("agent_plan", "Create and coordinate durable agent plans"),
    ]
    .into_iter()
    .map(|(id, description)| {
        RuntimeCapability::new(
            scriberig_app::CapabilityId::new(id).expect("static capability"),
            description,
            true,
        )
    })
    .collect()
}
