//! Defines the reconnect event replay method contract.

macro_rules! server_events_contract {
    () => {
        /// Returns public events after a reconnect cursor.
        ///
        /// - `event_kinds`: Event kind filters.
        /// - `after`: Last cursor already processed by the caller.
        ///
        /// Retained events newer than `after`.
        ///
        /// Returns an error when the replay cursor is no longer retained.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn events_after(
            &self,
            event_kinds: &[String],
            after: u64,
        ) -> Result<Vec<Event>, ApplicationError> {
            Ok(self
                .events(event_kinds)?
                .into_iter()
                .filter(|event| event.sequence > after)
                .collect())
        }
    };
}
