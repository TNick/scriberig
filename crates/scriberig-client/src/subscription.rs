//! Provides a reusable polling subscription over protocol event observations.

use scriberig_protocol::Event;

use crate::{Client, ClientError};

/// Owns the filter and client boundary for one event subscription.
///
/// # Public members
/// None.
///
/// # Private members
/// - `client`: Client used to retrieve event observations.
/// - `event_kinds`: Requested event kind filters.
pub struct EventSubscription<'a, T> {
    /// Client used to retrieve event observations.
    client: &'a Client<T>,
    /// Requested event kind filters.
    event_kinds: Vec<String>,
    /// Most recent cursor delivered to this subscriber.
    after: Option<u64>,
}

impl<'a, T: scriberig_protocol::ProtocolTransport> EventSubscription<'a, T> {
    /// Creates a subscription selecting all event kinds.
    ///
    /// # Parameters
    /// - `client`: Client used to poll event observations.
    ///
    /// # Returns
    /// An all-events subscription.
    #[must_use]
    pub fn all(client: &'a Client<T>) -> Self {
        Self {
            client,
            event_kinds: Vec::new(),
            after: None,
        }
    }

    /// Creates a subscription selecting named event kinds.
    ///
    /// # Parameters
    /// - `client`: Client used to poll event observations.
    /// - `event_kinds`: Event kind names accepted by the server.
    ///
    /// # Returns
    /// A filtered event subscription.
    #[must_use]
    pub fn filtered(client: &'a Client<T>, event_kinds: Vec<String>) -> Self {
        Self {
            client,
            event_kinds,
            after: None,
        }
    }

    /// Polls the server for currently available events.
    ///
    /// # Returns
    /// Stable event envelopes available at poll time.
    ///
    /// # Errors
    /// Returns the structured client or protocol failure.
    pub fn poll(&mut self) -> Result<Vec<Event>, ClientError> {
        let events = self
            .client
            .subscribe_after(self.event_kinds.clone(), self.after)?;
        if let Some(sequence) = events.iter().map(|event| event.sequence).max() {
            self.after = Some(sequence);
        }
        Ok(events)
    }
}

impl<T: scriberig_protocol::ProtocolTransport> Client<T> {
    /// Creates an all-events subscription.
    ///
    /// # Returns
    /// A subscription polling all available event kinds.
    #[must_use]
    pub fn event_subscription(&self) -> EventSubscription<'_, T> {
        EventSubscription::all(self)
    }
}
