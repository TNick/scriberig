//! Parses explicit terminal commands for preserved legacy selection recovery.

use crate::effect::Effect;
use scriberig_core::EntityId;
use scriberig_session::Session;

/// Parses a recovery inspection or operator mapping command.
///
/// - `command`: Complete slash command submitted by the operator.
/// - `session`: Exact currently loaded session.
///
/// The service effect that performs the requested recovery operation.
/// # Returns
/// The exact service request selected by the command.
///
/// # Errors
/// Returns usage guidance when the command is malformed.
pub(super) fn parse(command: &str, session: &Session) -> Result<Effect, String> {
    let mut parts = command.split_whitespace();
    let _name = parts.next();
    match parts.next() {
        None => Ok(Effect::LegacyRecoveryList {
            session: session.id,
            selection_revision: session.execution.revision,
            plan_revisions: session
                .plans
                .iter()
                .map(|plan| (plan.id, plan.revision))
                .collect(),
        }),
        Some("map") => parse_mapping(parts, session.id),
        Some(_) => Err(usage()),
    }
}

/// Parses an explicit path, revision, and active provider/model choice.
///
/// - `parts`: Remaining command arguments after `/recover map`.
/// - `session`: Exact currently loaded session.
///
/// A revision-checked mapping effect.
///
/// # Errors
/// Returns usage guidance when any required argument is missing or malformed.
fn parse_mapping<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    session: EntityId,
) -> Result<Effect, String> {
    let Some(path) = parts.next() else {
        return Err(usage());
    };
    let Some(revision) = parts.next().and_then(|value| value.parse::<u64>().ok()) else {
        return Err(usage());
    };
    let (Some(provider_id), Some(model_id)) = (parts.next(), parts.next()) else {
        return Err(usage());
    };
    let thinking_level = parts.next().map(str::to_owned);
    if parts.next().is_some() || path.is_empty() || provider_id.is_empty() || model_id.is_empty() {
        return Err(usage());
    }
    Ok(Effect::LegacyRecoveryMap {
        session,
        path: path.to_owned(),
        expected_revision: revision,
        provider_id: provider_id.to_owned(),
        model_id: model_id.to_owned(),
        thinking_level,
    })
}

/// Returns the supported recovery command syntax.
///
/// # Returns
/// Concise usage guidance for inspection and mapping.
fn usage() -> String {
    "Use /recover or /recover map <exact-path> <observed-revision> <provider> <model> [thinking]."
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::effect::Effect;
    use scriberig_session::{Session, SessionExecutionSelection};

    /// Creates a minimal session with an observable execution revision.
    ///
    /// # Returns
    /// A session fixture with revision four.
    fn session() -> Session {
        Session {
            id: scriberig_core::EntityId::from_bytes([1; 16]),
            branches: Vec::new(),
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: SessionExecutionSelection {
                revision: 4,
                ..Default::default()
            },
        }
    }

    /// Parses a session-level recovery mapping with an explicit revision.
    #[test]
    fn maps_exact_session_choice_with_observed_revision() {
        let session = session();
        assert_eq!(
            parse(
                "/recover map /execution 4 provider-a model-a high",
                &session
            )
            .unwrap(),
            Effect::LegacyRecoveryMap {
                session: session.id,
                path: "/execution".to_owned(),
                expected_revision: 4,
                provider_id: "provider-a".to_owned(),
                model_id: "model-a".to_owned(),
                thinking_level: Some("high".to_owned()),
            }
        );
    }

    /// Rejects malformed input rather than guessing an identity or revision.
    #[test]
    fn rejects_mapping_without_observed_revision_or_exact_arguments() {
        let session = session();
        assert!(parse("/recover map /execution provider-a model-a", &session).is_err());
        assert!(parse("/recover map /execution 1 provider-a", &session).is_err());
        assert!(parse("/recover map /execution 1 provider-a model-a x y", &session).is_err());
    }
}
