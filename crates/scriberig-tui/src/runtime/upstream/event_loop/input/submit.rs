//! Routes submitted composer text to product commands or the shared agent path.

use std::io;

use crate::action::Action;
use crate::service::TuiService;

use super::{
    ComposerInput, ControlKind, InputContext, TUI, palette, plan_assignment, queue_effects,
};

/// Handles one submitted composer draft and reports whether the loop should skip its tail.
///
/// - `context`: Shared application, TUI, and worker state for the submission.
/// - `prompt`: Exact submitted composer text.
///
/// # Returns
/// `true` when a command opened an overlay or completed without agent submission.
///
/// # Errors
/// Returns an error when state locking, diagnostics, or command presentation fails.
pub(super) fn handle<S: TuiService + Sync>(
    mut context: InputContext<'_, '_, S>,
    prompt: String,
) -> io::Result<bool> {
    *context.suppress_composer_restore = true;
    match prompt.trim() {
        "/model" => {
            let _ = context
                .input_sender
                .send(ComposerInput::OpenControl(ControlKind::Model));
            return Ok(true);
        }
        "/thinking" => {
            let _ = context
                .input_sender
                .send(ComposerInput::OpenControl(ControlKind::Thinking));
            return Ok(true);
        }
        "/assign" => {
            plan_assignment::open(&mut context, &prompt)?;
            *context.redraw = true;
            return Ok(true);
        }
        "/theme" => {
            let _ = context.input_sender.send(ComposerInput::OpenThemeSelector);
            return Ok(true);
        }
        value if value.starts_with("/theme ") => {
            if let Ok(mut state) = context.state.lock() {
                state.conversation_composer.error =
                    Some("Use /theme without arguments.".to_owned());
            }
            context.screen.request_render(false);
            return Ok(true);
        }
        _ => {}
    }
    if let Some((name, arguments)) =
        plugin_command_submission(&prompt, &context.service.plugin_commands())
    {
        let response = context.service.invoke_plugin_command(name, arguments);
        palette::present_plugin_response(&mut context, name, arguments, response)?;
        *context.redraw = true;
        return Ok(true);
    }
    let mut state = context
        .state
        .lock()
        .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
    state.conversation_composer.draft = prompt;
    let effects = crate::state::reduce(&mut state, Action::InputSubmitted);
    if let Some(log) = context.debug_log.as_deref_mut() {
        log.record_message("composer-submit", &state)?;
    }
    queue_effects(context.work_sender, context.pending_effects, effects);
    *context.should_quit = state.should_quit;
    *context.redraw = true;
    Ok(false)
}

/// Resolves one submitted slash command to an advertised non-built-in plugin command.
///
/// - `prompt`: Exact composer text submitted by the user.
/// - `commands`: Host-authorized command registrations from the upstream worker.
///
/// # Returns
/// The normalized command and raw trailing arguments when an exact registration matches.
fn plugin_command_submission<'a>(
    prompt: &'a str,
    commands: &[rpi_extensions::isolated::WorkerCommandSpec],
) -> Option<(&'a str, &'a str)> {
    let command_text = prompt.trim().strip_prefix('/')?;
    let (name, arguments) = command_text
        .split_once(char::is_whitespace)
        .map_or((command_text, ""), |(name, arguments)| {
            (name, arguments.trim_start())
        });
    let is_builtin = crate::slash_commands::find(&format!("/{name}")).is_some();
    let valid = crate::runtime::upstream::selector_factory::valid_plugin_command_name(name);
    (!is_builtin && valid && commands.iter().any(|command| command.name == name))
        .then_some((name, arguments))
}

#[cfg(test)]
mod tests {
    use super::plugin_command_submission;

    /// Recognizes advertised extension commands while retaining their arguments.
    #[test]
    fn plugin_slash_command_routes_exact_name_and_raw_arguments() {
        let commands = [rpi_extensions::isolated::WorkerCommandSpec {
            name: "fixture-status".to_owned(),
            description: "Fixture status".to_owned(),
        }];
        assert_eq!(
            plugin_command_submission("/fixture-status  alpha beta", &commands),
            Some(("fixture-status", "alpha beta")),
        );
        assert_eq!(plugin_command_submission("/fixture", &commands), None);
        assert_eq!(plugin_command_submission("/model", &commands), None);
    }
}
