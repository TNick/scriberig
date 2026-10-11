//! Defines server-side event subscription filtering over application events.

/// Describes the event kinds selected by one protocol subscription.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SubscriptionSpec {
    /// Requested protocol event names; empty selects every event.
    pub event_kinds: Vec<String>,
}

impl SubscriptionSpec {
    /// Creates a subscription selecting all public event kinds.
    #[must_use]
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn all() -> Self {
        Self::default()
    }

    /// Creates a subscription selecting the supplied event kinds.
    #[must_use]
    /// - `event_kinds`: Argument named `event_kinds`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn filtered(event_kinds: impl IntoIterator<Item = String>) -> Self {
        Self {
            event_kinds: event_kinds.into_iter().collect(),
        }
    }

    /// Reports whether a protocol event name is selected.
    #[must_use]
    /// - `event_kind`: Argument named `event_kind`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn accepts(&self, event_kind: &str) -> bool {
        self.event_kinds.is_empty()
            || self
                .event_kinds
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(event_kind))
    }
}
