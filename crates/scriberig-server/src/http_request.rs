//! Provides bounded HTTP header parsing shared by the local server adapter.

use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Returns a request header value without accepting malformed names.
pub(super) fn header_value<'a>(head: &'a str, wanted: &str) -> Option<&'a str> {
    head.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case(wanted).then_some(value.trim())
    })
}

/// Allows only the configured exact origin, never a wildcard.
///
/// # Parameters
/// - `super`: Internal server-module visibility.
/// - `head`: HTTP request headers.
/// - `configured`: Optional configured exact origin.
///
/// # Returns
/// The allowed origin when the request matches configuration.
pub(super) fn allowed_origin(head: &str, configured: Option<&str>) -> Option<String> {
    let origin = header_value(head, "origin")?;
    configured
        .filter(|value| !value.is_empty() && *value != "*")
        .filter(|value| *value == origin)
        .map(ToOwned::to_owned)
}

/// Returns a bounded request correlation identifier from headers or local state.
///
/// # Parameters
/// - `super`: Internal server-module visibility.
/// - `head`: HTTP request headers.
///
/// # Returns
/// A bounded request correlation identifier.
pub(super) fn request_id(head: &str) -> String {
    let supplied = head.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        (name.eq_ignore_ascii_case("x-request-id")).then_some(value.trim())
    });
    let value = supplied
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .map(ToOwned::to_owned);
    value.unwrap_or_else(|| format!("http-{}", REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)))
}
