//! Pins the upstream terminal-restoration contract behind the host event loop.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

use rpi_tui::{InputEvent, Terminal, TerminalInfo, TuiAltScreen};

/// Records terminal writes and mode transitions for lifecycle assertions.
///
/// # Public members
/// None.
///
/// # Private members
/// - `output`: Shared transcript of every terminal write and mode transition.
struct RecordingTerminal {
    /// Transcript of terminal writes and raw-mode transitions.
    output: Arc<Mutex<String>>,
}

impl RecordingTerminal {
    /// Appends one marker or escape sequence to the shared transcript.
    ///
    /// # Parameters
    /// - `marker`: Text representing terminal output or a state transition.
    fn record(&self, marker: &str) {
        if let Ok(mut output) = self.output.lock() {
            output.push_str(marker);
        }
    }
}

impl Terminal for RecordingTerminal {
    /// Reports a fixed deterministic terminal geometry.
    ///
    /// # Returns
    /// A stable 40x12 terminal description.
    fn info(&self) -> TerminalInfo {
        TerminalInfo {
            columns: 40,
            rows: 12,
            ..TerminalInfo::default()
        }
    }

    /// Records one terminal write.
    ///
    /// # Parameters
    /// - `data`: Escape or text bytes the runtime wrote.
    fn write(&self, data: &str) {
        self.record(data);
    }

    /// Records that the cursor was hidden.
    fn hide_cursor(&self) {
        self.record("<cursor-off>");
    }

    /// Records that the cursor was shown.
    fn show_cursor(&self) {
        self.record("<cursor-show>");
    }

    /// Records a cursor move.
    ///
    /// # Parameters
    /// - `_row`: Target zero-indexed row.
    /// - `_col`: Target zero-indexed column.
    fn move_cursor(&self, _row: usize, _col: usize) {}

    /// Records a screen clear.
    fn clear_screen(&self) {}

    /// Records a title change.
    ///
    /// # Parameters
    /// - `_title`: Requested terminal title.
    fn set_title(&self, _title: &str) {}

    /// Records mouse reporting being enabled.
    fn enable_mouse(&self) {}

    /// Records mouse reporting being disabled.
    fn disable_mouse(&self) {}

    /// Records entry into terminal raw mode.
    fn enter_raw_mode(&self) {
        self.record("<raw-on>");
    }

    /// Records exit from terminal raw mode.
    fn exit_raw_mode(&self) {
        self.record("<raw-off>");
    }

    /// Reports that the geometry did not change.
    fn refresh_size(&self) {}

    /// Enters raw mode without starting an input reader thread.
    ///
    /// # Parameters
    /// - `_on_input`: Upstream input callback the real terminal would invoke.
    /// - `_on_resize`: Upstream resize callback the real terminal would invoke.
    fn start(
        &self,
        _on_input: Box<dyn Fn(InputEvent) + Send + Sync>,
        _on_resize: Box<dyn Fn() + Send + Sync>,
    ) {
        self.enter_raw_mode();
    }

    /// Records the terminal restoration request.
    fn stop(&self) {
        self.record("<terminal-stop>");
    }

    /// Reports that this recording terminal is not a real TTY.
    ///
    /// # Returns
    /// `false`.
    fn is_tty(&self) -> bool {
        false
    }

    /// Records a progress indicator request.
    ///
    /// # Parameters
    /// - `_active`: Whether a progress indicator was requested.
    fn set_progress(&self, _active: bool) {}

    /// Flushes the transcript.
    fn flush(&self) {}
}

/// Keeps the terminal restoration guarantee for a panic inside the host tick.
///
/// `runtime/upstream/event_loop.rs` runs the whole `ScribeRig` application tick
/// inside `TuiAltScreen::run_until_with_tick`, so a `ScribeRig` panic must still
/// leave raw mode, the alternate screen, and the hardware cursor restored. This
/// pins that contract for the pinned fork revision, where upstream's own
/// equivalent case asserts a raw-mode marker that its lifecycle restore path no
/// longer emits.
#[test]
fn run_until_with_tick_restores_the_terminal_after_a_host_tick_panic() {
    let output = Arc::new(Mutex::new(String::new()));
    let screen = Arc::new(TuiAltScreen::new(
        Box::new(RecordingTerminal {
            output: Arc::clone(&output),
        }),
        true,
        None,
    ));

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        screen.run_until_with_tick(|| panic!("host tick panic"));
    }));

    assert!(
        outcome.is_err(),
        "a host tick panic must still reach the `ScribeRig` caller"
    );
    let transcript = output.lock().expect("terminal transcript lock");
    assert!(
        transcript.contains("<terminal-stop>"),
        "the upstream lifecycle must stop the terminal after a host panic: {transcript}"
    );
    assert!(
        transcript.contains("\x1b[?1049l"),
        "the alternate screen must be left after a host panic: {transcript}"
    );
    assert!(
        transcript.contains("\x1b[?25h"),
        "the hardware cursor must be restored after a host panic: {transcript}"
    );
}
