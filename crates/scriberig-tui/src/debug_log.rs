//! Writes content-redacted diagnostics for interactive TUI debugging.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{event::TuiEvent, state::TuiState};

/// Owns the optional TUI diagnostic log file.
///
/// Public members: None.
///
/// Private members
/// - `file`: Append-only diagnostic destination.
/// - `path`: Destination recorded at startup.
pub(crate) struct TuiDebugLog {
    /// Append-only diagnostic destination.
    file: File,
    /// Destination recorded at startup.
    path: PathBuf,
}

impl TuiDebugLog {
    /// Opens an explicit log path or a timestamped path under the TUI log folder.
    ///
    /// - `requested_path`: Optional caller-selected log destination.
    ///
    /// The initialized log writer.
    ///
    /// # Returns
    /// The initialized diagnostic writer.
    ///
    /// # Errors
    /// Returns an I/O error when its parent directory or file cannot be created.
    pub(crate) fn open(requested_path: Option<&Path>) -> io::Result<Self> {
        let path = requested_path.map_or_else(default_path, Path::to_path_buf);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let mut log = Self { file, path };
        writeln!(
            log.file,
            "time_ms={} event=debug-log-start path={}",
            timestamp_millis(),
            log.path.display()
        )?;
        log.file.flush()?;
        Ok(log)
    }

    /// Records a translated terminal event and its immediate reducer effects.
    ///
    /// - `event`: Translated event, rendered without user text.
    /// - `effect_count`: Number of effects returned by the reducer.
    /// - `state`: Resulting TUI state, summarized without composer content.
    ///
    /// # Returns
    /// Unit after the event has been recorded.
    ///
    /// # Errors
    /// Returns an I/O error when the diagnostic destination cannot be written.
    pub(crate) fn record_event(
        &mut self,
        event: &TuiEvent,
        effect_count: usize,
        state: &TuiState,
    ) -> io::Result<()> {
        let event = event_label(event);
        writeln!(
            self.file,
            "time_ms={} event=terminal input={event} effects={effect_count} draft_bytes={} pending={} overlay={} model_library={} operation_error={} error_present={}",
            timestamp_millis(),
            state.conversation_composer.draft.len(),
            state.conversation_composer.pending,
            overlay_label(state),
            model_library_label(&state.model_library),
            operation_error_label(state),
            state.conversation_composer.error.is_some(),
        )?;
        self.file.flush()
    }

    /// Records state after an asynchronous application result is reduced.
    ///
    /// - `message`: Safe description of the returned runtime message.
    /// - `state`: Resulting TUI state, summarized without conversation content.
    ///
    /// # Returns
    /// Unit after the message has been recorded.
    ///
    /// # Errors
    /// Returns an I/O error when the diagnostic destination cannot be written.
    pub(crate) fn record_message(&mut self, message: &str, state: &TuiState) -> io::Result<()> {
        writeln!(
            self.file,
            "time_ms={} event=runtime-message message={message} draft_bytes={} pending={} overlay={} model_library={} operation_error={} error_present={}",
            timestamp_millis(),
            state.conversation_composer.draft.len(),
            state.conversation_composer.pending,
            overlay_label(state),
            model_library_label(&state.model_library),
            operation_error_label(state),
            state.conversation_composer.error.is_some(),
        )?;
        self.file.flush()
    }
}

/// Returns the current Unix timestamp in milliseconds.
///
/// Unix milliseconds, or zero when the system clock predates the Unix epoch.
/// # Returns
/// The operation result.
fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Resolves a timestamped log path using UTC calendar time.
///
/// Timestamped path under `playground/tui-logs`.
/// # Returns
/// The operation result.
fn default_path() -> PathBuf {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = seconds / 86_400;
    let day_seconds = seconds % 86_400;
    let (year, month, day) = civil_date(i64::try_from(days).unwrap_or(i64::MAX));
    let hour = day_seconds / 3_600;
    let minute = day_seconds % 3_600 / 60;
    let second = day_seconds % 60;
    PathBuf::from(format!(
        "playground/tui-logs/{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}.log"
    ))
}

/// Converts days since the Unix epoch to a Gregorian calendar date.
///
/// - `days`: Whole UTC days since 1970-01-01.
///
/// Year, month, and day components.
/// # Returns
/// The operation result.
fn civil_date(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

/// Formats a terminal event without recording typed or pasted content.
///
/// - `event`: Translated terminal event to describe.
///
/// Content-safe event description.
/// # Returns
/// The operation result.
fn event_label(event: &TuiEvent) -> String {
    match event {
        TuiEvent::Key(key) => format!(
            "key:{}:ctrl={}:alt={}:shift={}",
            key_code_label(key.code),
            key.modifiers.control,
            key.modifiers.alt,
            key.modifiers.shift
        ),
        TuiEvent::Paste(text) => format!("paste:bytes={}", text.len()),
        TuiEvent::Resize { width, height } => format!("resize:{width}x{height}"),
        TuiEvent::Tick => "tick".to_owned(),
        TuiEvent::Shutdown => "shutdown".to_owned(),
        TuiEvent::Application(_) => "application".to_owned(),
    }
}

/// Returns a content-safe label for a translated key code.
///
/// - `code`: Logical terminal key code.
///
/// Key name without printable character content.
/// # Returns
/// The operation result.
fn key_code_label(code: crate::event::KeyCode) -> &'static str {
    use crate::event::KeyCode;

    match code {
        KeyCode::Char(_) => "character",
        KeyCode::Esc => "escape",
        KeyCode::Enter => "enter",
        KeyCode::Tab => "tab",
        KeyCode::Backspace => "backspace",
        KeyCode::Delete => "delete",
        KeyCode::Home => "home",
        KeyCode::End => "end",
        KeyCode::Up => "up",
        KeyCode::Down => "down",
        KeyCode::Left => "left",
        KeyCode::Right => "right",
        KeyCode::Help => "help",
        KeyCode::Other => "other",
    }
}

/// Describes the current overlay without exposing its query text.
///
/// - `state`: Current TUI state.
///
/// Overlay name or `none`.
/// # Returns
/// The operation result.
fn overlay_label(state: &TuiState) -> &'static str {
    match state.overlay {
        None => "none",
        Some(crate::overlay::Overlay::Help) => "help",
        Some(crate::overlay::Overlay::SessionList(_)) => "session-list",
        Some(crate::overlay::Overlay::Palette(_)) => "palette",
        Some(crate::overlay::Overlay::Input(_)) => "input",
        Some(crate::overlay::Overlay::Confirm(_)) => "confirm",
        Some(crate::overlay::Overlay::Composer(_)) => "composer",
    }
}

/// Describes model-library state and enabled profile counts.
///
/// - `state`: Credential-safe model-library loading state.
///
/// Safe loading-state label.
/// # Returns
/// The operation result.
fn model_library_label(
    state: &crate::inspector::LoadState<scriberig_protocol::ModelLibraryResponse>,
) -> String {
    match state {
        crate::inspector::LoadState::NotRequested => "not-requested".to_owned(),
        crate::inspector::LoadState::Loading { .. } => "loading".to_owned(),
        crate::inspector::LoadState::Failed { category, .. } => format!("failed:{category}"),
        crate::inspector::LoadState::Ready(library) => {
            format!("ready:models={}", library.models.len())
        }
    }
}

/// Returns the latest bounded, sanitized runtime error diagnostic.
///
/// - `state`: Current TUI state.
///
/// User-facing diagnostic or `none`.
/// # Returns
/// The operation result.
fn operation_error_label(state: &TuiState) -> String {
    let diagnostic = state
        .operation_error
        .as_ref()
        .map(|(category, message)| format!("{category}: {message}"))
        .or_else(|| state.conversation_composer.error.clone());
    diagnostic.map_or_else(
        || "none".to_owned(),
        |message| {
            crate::inspector::sanitize(&message)
                .chars()
                .take(512)
                .collect()
        },
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::{event::TuiEvent, state::TuiState};

    use super::{TuiDebugLog, default_path, timestamp_millis};

    /// Verifies nested log directories are created and text content is redacted.
    #[test]
    fn creates_log_parent_and_records_content_safe_key_diagnostics() {
        let timestamp_path = default_path();
        let timestamp = timestamp_path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".log"))
            .expect("timestamped log name");
        let directory = PathBuf::from("playground/agentic-work")
            .join(format!("{timestamp} tui log {}", std::process::id()));
        let log_path = directory.join("nested/session.log");
        let mut state = TuiState::default();
        state.conversation_composer.draft = "private prompt text".to_owned();
        state.conversation_composer.error = Some("provider request failed".to_owned());
        {
            let mut log = TuiDebugLog::open(Some(&log_path)).expect("open nested log");
            log.record_event(
                &TuiEvent::Key(crate::event::Key {
                    code: crate::event::KeyCode::Enter,
                    modifiers: crate::event::Modifiers::default(),
                }),
                1,
                &state,
            )
            .expect("record key event");
        }

        let contents = fs::read_to_string(&log_path).expect("read log");
        assert!(contents.contains("input=key:enter"));
        assert!(contents.contains("draft_bytes=19"));
        assert!(contents.contains("operation_error=provider request failed"));
        assert!(!contents.contains("private prompt text"));
        assert!(timestamp_millis() > 0);
        fs::remove_dir_all(directory).expect("remove test log directory");
    }
}
