//! Verifies safe upstream model protocol metadata contracts.

use super::*;

/// Creates a safe upstream model metadata fixture.
///
/// # Returns
/// A stable metadata fixture containing only safe display fields.
pub(super) fn model_metadata_snapshot() -> PublicModelMetadataSnapshot {
    PublicModelMetadataSnapshot {
        schema_version: "rpi-safe-model-snapshot-v1".to_owned(),
        api_id: "anthropic-messages".to_owned(),
        reasoning: true,
        input: vec![
            PublicModelInputModality::Text,
            PublicModelInputModality::Image,
        ],
        cost: PublicModelCostSnapshot {
            input: 3.0,
            output: 15.0,
            cache_read: 0.3,
            cache_write: 3.75,
            tiers: vec![PublicModelCostTierSnapshot {
                input: 2.0,
                output: 10.0,
                cache_read: 0.2,
                cache_write: 2.5,
                input_tokens_above: 100_000,
            }],
        },
        cost_configured: true,
        context_window: 200_000,
        max_tokens: 8_192,
    }
}

/// Verifies model display DTOs omit request secrets and excluded metadata.
#[test]
fn model_snapshot_dto_contains_only_safe_display_metadata() {
    let descriptor = PublicModelDescriptor {
        id: model_row_id("provider-a", "wire-model-a"),
        display_name: "Model A".to_owned(),
        provider_id: "provider-a".to_owned(),
        provider_model_id: "wire-model-a".to_owned(),
        thinking_levels: vec!["extended".to_owned()],
        metadata_snapshot: model_metadata_snapshot(),
    };
    let serialized = serde_json::to_value(descriptor).expect("serialize safe model row");
    let metadata = &serialized["metadata_snapshot"];

    assert_eq!(
        model_row_id("provider-a", "wire-model-a"),
        "rpi1:10:provider-a12:wire-model-a"
    );
    assert_eq!(serialized["id"], "rpi1:10:provider-a12:wire-model-a");
    assert_eq!(serialized["provider_id"], "provider-a");
    assert_eq!(serialized["provider_model_id"], "wire-model-a");
    assert_eq!(
        serialized["thinking_levels"],
        serde_json::json!(["extended"])
    );
    assert!(serialized.get("providerId").is_none());
    assert!(serialized.get("providerModelId").is_none());
    assert_eq!(metadata["schema_version"], "rpi-safe-model-snapshot-v1");
    assert_eq!(metadata["reasoning"], true);
    assert_eq!(
        metadata["cost"]["tiers"],
        serde_json::json!([{
            "input": 2.0,
            "output": 10.0,
            "cache_read": 0.2,
            "cache_write": 2.5,
            "input_tokens_above": 100_000,
        }])
    );
    for excluded in [
        "endpoint",
        "base_url",
        "headers",
        "sampling_parameters",
        "provider_options",
        "compatibility",
        "excluded_execution_fields",
    ] {
        assert!(metadata.get(excluded).is_none(), "unexpected {excluded}");
    }
}

/// Verifies a retired profile id cannot stand in for the upstream provider id.
#[test]
fn model_snapshot_requires_upstream_provider_identity() {
    let value = serde_json::json!({
        "id": "model-a",
        "display_name": "Model A",
        "provider_profile": "retired-profile",
        "provider_model_id": "wire-model-a",
        "thinking_levels": ["extended"],
        "metadata_snapshot": {
            "schema_version": "rpi-safe-model-snapshot-v1",
            "api_id": "openai-completions",
            "reasoning": true,
            "input": ["text"],
            "cost": {
                "input": 1.0,
                "output": 2.0,
                "cache_read": 0.0,
                "cache_write": 0.0
            },
            "cost_configured": true,
            "context_window": 8192,
            "max_tokens": 2048
        }
    });

    assert!(serde_json::from_value::<PublicModelDescriptor>(value).is_err());
}
