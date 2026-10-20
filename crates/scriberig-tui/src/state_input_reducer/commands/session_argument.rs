//! Parses exact session identities from local slash commands.

/// Parses exactly one stable session identity after a local command.
///
/// - `command`: Complete submitted slash command.
/// - `prefix`: Command spelling without its argument.
///
/// The exact session identity.
/// # Returns
/// The parsed session identity.
/// # Errors
/// Returns usage guidance when the argument is missing, extra, or malformed.
pub(super) fn parse_session_id(
    command: &str,
    prefix: &str,
) -> Result<scriberig_core::EntityId, &'static str> {
    let Some(arguments) = command.strip_prefix(prefix) else {
        return Err("Invalid session command.");
    };
    let mut values = arguments.split_whitespace();
    let Some(session) = values.next() else {
        return Err("Provide a session ID from /sessions.");
    };
    if values.next().is_some() {
        return Err("Provide exactly one session ID.");
    }
    session
        .parse()
        .map_err(|_| "Session ID is invalid; use an ID shown by /sessions.")
}
