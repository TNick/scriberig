//! Defines lifecycle event read methods for the protocol application adapter.

macro_rules! server_application_event_methods {
    () => {
        /// Exposes buffered lifecycle events as stable protocol envelopes.
        /// - `event_kinds`: Argument named `event_kinds`.
        ///
        ///
        /// Returns an error when the operation cannot complete.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn events(&self, event_kinds: &[String]) -> Result<Vec<Event>, ApplicationError> {
            Ok(self
                .operations()
                .events_with_sequences()
                .into_iter()
                .map(|(sequence, event)| {
                    let mut event = event_from_application(event);
                    event.sequence = sequence;
                    event
                })
                .filter(|event| event_matches(event, event_kinds))
                .collect())
        }

        /// Returns retained application events after a reconnect cursor.
        ///
        /// - `event_kinds`: Event kind filters.
        /// - `after`: Last cursor already processed by the caller.
        ///
        /// Events newer than `after`.
        ///
        /// Returns `cursor_too_old` when replay history has expired.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn events_after(
            &self,
            event_kinds: &[String],
            after: u64,
        ) -> Result<Vec<Event>, ApplicationError> {
            filter_events_after(self.events(event_kinds)?, after)
        }
    };
}
