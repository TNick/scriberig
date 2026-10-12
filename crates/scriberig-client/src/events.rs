//! Provides a cancellable blocking SSE subscription for the Rust client.

use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use scriberig_protocol::Event;

/// Owns a reconnecting remote protocol event stream.
pub struct HttpEventStream {
    stop: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
    worker: Option<JoinHandle<()>>,
}

/// Owns immutable connection settings for one event worker.
struct StreamConfig {
    /// Remote protocol host.
    host: String,
    /// Remote protocol port.
    port: u16,
    /// Remote protocol path.
    path: String,
    /// Socket timeout.
    timeout: Duration,
}

impl HttpEventStream {
    /// Starts a reconnecting SSE worker.
    ///
    /// # Parameters
    /// - host: Remote protocol host.
    /// - port: Remote protocol port.
    /// - path: Base protocol path.
    /// - timeout: Socket timeout.
    /// - after: Last consumed event cursor.
    /// - callback: Handler for decoded protocol events.
    ///
    /// # Returns
    /// A handle that stops the worker on close or drop.
    ///
    /// # Errors
    /// Returns an error when the worker cannot be created.
    pub(crate) fn start<F>(
        host: String,
        port: u16,
        path: String,
        timeout: Duration,
        after: Option<u64>,
        callback: F,
    ) -> io::Result<Self>
    where
        F: Fn(Event) + Send + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));
        let worker_stop = Arc::clone(&stop);
        let worker_error = Arc::clone(&error);
        let worker = thread::Builder::new()
            .name("scriberig-events".to_owned())
            .spawn(move || {
                run(
                    &StreamConfig {
                        host,
                        port,
                        path,
                        timeout,
                    },
                    after,
                    &worker_stop,
                    &worker_error,
                    callback,
                );
            })?;
        Ok(Self {
            stop,
            error,
            worker: Some(worker),
        })
    }

    /// Stops the worker and waits for its network task to exit.
    ///
    /// # Parameters
    /// - `self`: Event stream handle to close.
    pub fn close(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }

    /// Returns and clears the latest transport failure.
    ///
    /// # Returns
    /// The most recent connection or decoding error, when one occurred.
    #[must_use]
    pub fn take_error(&self) -> Option<String> {
        self.error.lock().ok()?.take()
    }
}

impl Drop for HttpEventStream {
    /// Stops the worker when the subscription handle is dropped.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Runs the bounded reconnect loop for one event stream.
fn run<F>(
    config: &StreamConfig,
    mut after: Option<u64>,
    stop: &Arc<AtomicBool>,
    error: &Arc<Mutex<Option<String>>>,
    callback: F,
) where
    F: Fn(Event),
{
    while !stop.load(Ordering::Acquire) {
        match connect_and_read(
            &config.host,
            config.port,
            &config.path,
            config.timeout,
            after,
            stop,
            &callback,
        ) {
            Ok(cursor) => after = cursor.or(after),
            Err(message) => {
                if let Ok(mut slot) = error.lock() {
                    *slot = Some(message);
                }
            }
        }
        if !stop.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(250));
        }
    }
}

/// Reads one bounded SSE connection and returns its newest cursor.
fn connect_and_read<F>(
    host: &str,
    port: u16,
    path: &str,
    timeout: Duration,
    after: Option<u64>,
    stop: &AtomicBool,
    callback: &F,
) -> Result<Option<u64>, String>
where
    F: Fn(Event),
{
    let address = format!("{host}:{port}");
    let mut stream = TcpStream::connect(&address).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(timeout.min(Duration::from_millis(250))))
        .map_err(|error| error.to_string())?;
    let request_path = after.map_or_else(
        || format!("{path}/events"),
        |cursor| format!("{path}/events?after={cursor}"),
    );
    write!(
        stream,
        "GET {request_path} HTTP/1.1\r\nHost: {host}\r\nAccept: text/event-stream\r\nConnection: close\r\n\r\n"
    )
    .map_err(|error| error.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let count = reader
        .read_line(&mut line)
        .map_err(|error| error.to_string())?;
    if count == 0 || !line.starts_with("HTTP/1.1 200 ") {
        return Err(format!("event stream rejected: {}", line.trim()));
    }
    loop {
        line.clear();
        let count = reader
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if count == 0 {
            return Err("event stream closed before headers".to_owned());
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
    }
    let mut cursor = after;
    let mut event_id = None;
    let mut data = String::new();
    loop {
        if stop.load(Ordering::Acquire) {
            return Ok(cursor);
        }
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return Ok(cursor),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::TimedOut => continue,
            Err(error) => return Err(error.to_string()),
        }
        let value = line.trim_end_matches(['\r', '\n']);
        if value.is_empty() {
            if let Some(payload) = take_event_payload(&mut data) {
                let event: Event =
                    serde_json::from_str(&payload).map_err(|error| error.to_string())?;
                if let Some(id) = event_id.take() {
                    cursor = Some(cursor.map_or(id, |current| current.max(id)));
                }
                callback(event);
            }
            continue;
        }
        if let Some(value) = value.strip_prefix("id:") {
            event_id = value.trim().parse::<u64>().ok();
        } else if let Some(value) = value.strip_prefix("data:") {
            if data.len() + value.len() <= 1_048_576 {
                data.push_str(value.trim_start());
            } else {
                return Err("event payload exceeded 1 MiB".to_owned());
            }
        }
    }
}

/// Takes a completed SSE data payload while ignoring heartbeat frames.
///
/// # Parameters
/// - `data`: Mutable accumulated SSE data payload.
///
/// # Returns
/// The completed payload, or `None` for an empty heartbeat frame.
fn take_event_payload(data: &mut String) -> Option<String> {
    if data.is_empty() {
        return None;
    }
    Some(std::mem::take(data))
}

#[cfg(test)]
mod tests {
    use super::take_event_payload;

    /// Verifies empty SSE frames do not become semantic events.
    #[test]
    fn ignores_empty_payloads() {
        let mut data = String::new();
        assert!(take_event_payload(&mut data).is_none());
    }

    /// Verifies completed SSE payloads are consumed exactly once.
    #[test]
    fn consumes_payload_once() {
        let mut data = " {\"sequence\":1} ".to_owned();
        assert_eq!(
            take_event_payload(&mut data).as_deref(),
            Some(" {\"sequence\":1} ")
        );
        assert_eq!(data, "");
        assert!(take_event_payload(&mut data).is_none());
    }
}
