//! Converts effective projection style evidence into a bounded terminal summary.

/// Converts the application-owned effective style to its reusable wire response.
///
/// - `style`: Shared resolved style with provenance and fidelity.
///
/// # Returns
/// The equivalent transport response without losing source evidence.
pub(crate) fn from_application(
    style: scriberig_app::EffectiveProjectionStyle,
) -> scriberig_protocol::ProjectionStyleResolveResponse {
    use scriberig_protocol as wire;
    use sr_projection::{ProjectionFidelity, StyleRef};

    let reference = match style.resolved.reference() {
        StyleRef::Value(_) => wire::ProjectionStyleReference::Inline,
        StyleRef::Revision { entity, revision } => wire::ProjectionStyleReference::Revision {
            entity: entity.to_string(),
            revision: revision.to_string(),
        },
    };
    wire::ProjectionStyleResolveResponse {
        purpose: match style.purpose {
            sr_projection::ProjectionPurpose::InteractiveView => {
                wire::ProjectionStylePurpose::InteractiveView
            }
            sr_projection::ProjectionPurpose::SharedMaterialization => {
                wire::ProjectionStylePurpose::SharedMaterialization
            }
            sr_projection::ProjectionPurpose::ExplicitExport => {
                wire::ProjectionStylePurpose::ExplicitExport
            }
        },
        reference,
        value: style.resolved.value().as_bytes().to_vec(),
        schema: style
            .resolved
            .schema()
            .map(|schema| wire::ProjectionStyleSchemaReference {
                adapter_id: schema.adapter_id().to_owned(),
                version: schema.version().to_owned(),
            }),
        applied: style
            .applied
            .into_iter()
            .map(|source| wire::ProjectionStyleSource {
                kind: source.kind.to_string(),
                identity: source.identity,
                revision: source.revision.map(|revision| revision.to_string()),
                generation: source.generation,
                value: source.value.as_bytes().to_vec(),
            })
            .collect(),
        fidelity: wire::ProjectionStyleFidelityReport {
            fidelity: match style.fidelity.fidelity {
                ProjectionFidelity::LosslessEditable => {
                    wire::ProjectionStyleFidelity::LosslessEditable
                }
                ProjectionFidelity::LossAwareEditable => {
                    wire::ProjectionStyleFidelity::LossAwareEditable
                }
                ProjectionFidelity::ReadOnlyLossy => wire::ProjectionStyleFidelity::ReadOnlyLossy,
            },
            omitted_categories: style.fidelity.omitted_categories,
            explanation: style.fidelity.explanation,
        },
    }
}

/// Builds a bounded human-readable summary of resolved style evidence.
///
/// - `resolution`: Exact style resolution to show in the shell status.
///
/// # Returns
/// A summary naming the schema, fidelity, omitted categories, and applied sources.
pub(crate) fn summary(resolution: &scriberig_protocol::ProjectionStyleResolveResponse) -> String {
    const MAX_VALUE_CHARS: usize = 160;
    const MAX_SOURCES: usize = 12;
    let schema = resolution.schema.as_ref().map_or_else(
        || "unschematized".to_owned(),
        |schema| {
            format!(
                "{} v{}",
                bounded_safe_text(&schema.adapter_id, 64),
                bounded_safe_text(&schema.version, 32)
            )
        },
    );
    let decoded_value = String::from_utf8_lossy(&resolution.value);
    let mut value = crate::inspector::sanitize(&decoded_value);
    if value.chars().count() > MAX_VALUE_CHARS {
        value = value.chars().take(MAX_VALUE_CHARS).collect();
        value.push('…');
    }
    let fidelity = match resolution.fidelity.fidelity {
        scriberig_protocol::ProjectionStyleFidelity::LosslessEditable => "lossless editable",
        scriberig_protocol::ProjectionStyleFidelity::LossAwareEditable => "loss-aware editable",
        scriberig_protocol::ProjectionStyleFidelity::ReadOnlyLossy => "read-only lossy",
    };
    let sources = resolution
        .applied
        .iter()
        .take(MAX_SOURCES)
        .map(|source| {
            format!(
                "{}:{}",
                bounded_safe_text(&source.kind, 48),
                bounded_safe_text(&source.identity, 80)
            )
        })
        .collect::<Vec<_>>();
    let omitted = if resolution.fidelity.omitted_categories.is_empty() {
        "none".to_owned()
    } else {
        resolution
            .fidelity
            .omitted_categories
            .iter()
            .take(MAX_SOURCES)
            .map(|category| bounded_safe_text(category, 64))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let explanation = resolution.fidelity.explanation.as_deref().map_or_else(
        || "no additional fidelity note".to_owned(),
        |text| bounded_safe_text(text, 240),
    );
    let extra_sources = resolution.applied.len().saturating_sub(MAX_SOURCES);
    format!(
        "Effective Rust source style · {schema} · value: {value} · {fidelity}; sources: {}; omitted: {omitted}; {explanation}",
        if sources.is_empty() {
            "none".to_owned()
        } else {
            format!(
                "{}{}",
                sources.join(" → "),
                if extra_sources > 0 {
                    format!(" → {extra_sources} more")
                } else {
                    String::new()
                }
            )
        },
    )
}

/// Sanitizes and bounds adapter-owned text before terminal presentation.
///
/// - `value`: Adapter-owned label or explanation.
/// - `maximum`: Maximum number of Unicode scalar values to retain.
///
/// # Returns
/// Control-free bounded text with an ellipsis when truncated.
fn bounded_safe_text(value: &str, maximum: usize) -> String {
    let safe = crate::inspector::sanitize(value);
    let mut chars = safe.chars();
    let mut bounded = chars.by_ref().take(maximum).collect::<String>();
    if chars.next().is_some() {
        bounded.push('…');
    }
    bounded
}

#[cfg(test)]
mod tests {
    use super::{from_application, summary};
    use scriberig_app::EffectiveProjectionStyle;
    use sr_projection::{
        FidelityAssessment, ProjectionPurpose, ResolvedStyle, StyleSource, StyleSourceKind,
        StyleValue,
    };

    /// Preserves source chain, resolved bytes, and fidelity across the TUI boundary.
    #[test]
    fn application_style_conversion_preserves_resolution_evidence() {
        let response = from_application(EffectiveProjectionStyle {
            purpose: ProjectionPurpose::InteractiveView,
            resolved: ResolvedStyle::inline(StyleValue::new(b"indent=tabs".to_vec())),
            applied: vec![StyleSource::new(
                StyleSourceKind::UserPreference,
                "principal-a",
                None,
                StyleValue::new(b"indent=tabs".to_vec()),
            )],
            fidelity: FidelityAssessment::loss_aware(
                vec!["ordinary-comments".to_owned()],
                Some("comment lineage retained".to_owned()),
            ),
        });
        assert_eq!(response.value, b"indent=tabs");
        assert_eq!(response.applied[0].kind, "user-preference");
        assert_eq!(response.applied[0].identity, "principal-a");
        assert_eq!(
            response.fidelity.fidelity,
            scriberig_protocol::ProjectionStyleFidelity::LossAwareEditable
        );
        assert_eq!(response.fidelity.omitted_categories, ["ordinary-comments"]);
        let text = summary(&response);
        assert!(text.contains("value: indent=tabs"));
        assert!(text.contains("loss-aware editable"));
        assert!(text.contains("ordinary-comments"));
    }

    /// Presents the shared cross-surface fixture's exact style and fidelity evidence.
    #[test]
    fn shared_web_tui_fixture_preserves_style_and_fidelity() {
        let response: scriberig_protocol::ProjectionStyleResolveResponse = serde_json::from_str(
            include_str!("../../../../tests/fixtures/projection-style-rust.json"),
        )
        .expect("decode shared projection style fixture");

        assert_eq!(response.value, b"indent=tabs,indent-width=2");
        assert_eq!(response.applied.len(), 1);
        assert_eq!(response.applied[0].identity, "fixture-user");
        assert_eq!(response.applied[0].generation, Some(2));
        assert_eq!(
            response.fidelity.fidelity,
            scriberig_protocol::ProjectionStyleFidelity::LosslessEditable
        );
        assert_eq!(
            response.fidelity.omitted_categories,
            [] as [std::string::String; 0]
        );
        let text = summary(&response);
        assert!(text.contains("value: indent=tabs,indent-width=2"));
        assert!(text.contains("lossless editable"));
        assert!(text.contains("fixture-user"));
    }

    /// Removes terminal control characters before including adapter style text in status.
    #[test]
    fn summary_sanitizes_and_bounds_style_value() {
        let mut response = from_application(EffectiveProjectionStyle {
            purpose: ProjectionPurpose::InteractiveView,
            resolved: ResolvedStyle::inline(StyleValue::new(
                [vec![b'a'; 180], b"\x1b[31m".to_vec()].concat(),
            )),
            applied: Vec::new(),
            fidelity: FidelityAssessment::lossless(),
        });
        response.schema = None;
        let text = summary(&response);
        assert!(!text.contains('\u{1b}'));
        assert!(text.contains('…'));
        assert!(text.len() < 500);
    }
}
