//! Adds cancellation observation to the OpenAI-compatible provider boundary.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use super::OpenAiCompatibleProvider;
use crate::provider::{ModelRequest, ModelResponse, ProviderAdapter, ProviderError};

impl OpenAiCompatibleProvider {
    /// Completes a request while observing shared runtime cancellation.
    ///
    /// - `self`: Configured provider adapter.
    /// - `request`: Provider-neutral model request.
    /// - `cancelled`: Shared cancellation flag for the active operation.
    ///
    /// A normalized provider response.
    ///
    /// Returns `Cancelled` when cancellation wins, or the classified provider
    /// failure produced by the underlying request.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn complete_with_cancellation(
        &self,
        request: &ModelRequest,
        cancelled: &AtomicBool,
    ) -> Result<ModelResponse, ProviderError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(ProviderError::Cancelled);
        }
        let provider = self.clone();
        let request = request.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let _ = sender.send(provider.complete(&request));
        });
        loop {
            if cancelled.load(Ordering::Acquire) {
                return Err(ProviderError::Cancelled);
            }
            match receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(ProviderError::Transport(
                        "provider worker disconnected".to_owned(),
                    ));
                }
            }
        }
    }
}
