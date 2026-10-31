//! Owns session-scoped plugin component registrations from isolated workers.

use std::collections::BTreeMap;
use std::sync::Mutex;

static NEXT_OWNER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Routes worker-local component owners into one host UI mailbox.
///
/// Each dispatcher represents one worker process. Its owner mapping keeps
/// plugin IDs unique across workers and removes their panels when the worker
/// composition is dropped.
///
/// # Public members
/// - [`PluginComponentDispatcher::new`]: Binds one upstream mailbox.
/// - [`PluginComponentDispatcher::dispatch`]: Applies a worker component call.
///
/// # Private members
/// - `mailbox`: Shared upstream component registry.
/// - `owners`: Worker-local to host-global owner mapping.
pub struct PluginComponentDispatcher {
    /// Shared upstream component registry.
    mailbox: rpi_extensions::ExtensionStatusMailbox,
    /// Worker-local to host-global owner mapping.
    owners: Mutex<BTreeMap<u64, u64>>,
}

impl PluginComponentDispatcher {
    /// Creates a dispatcher for one isolated worker process.
    ///
    /// # Parameters
    /// - `mailbox`: Product session's upstream component registry.
    ///
    /// # Returns
    /// A dispatcher whose registrations are removed when it is dropped.
    #[must_use]
    pub fn new(mailbox: rpi_extensions::ExtensionStatusMailbox) -> Self {
        Self {
            mailbox,
            owners: Mutex::new(BTreeMap::new()),
        }
    }

    /// Applies one isolated worker component request.
    ///
    /// # Parameters
    /// - `arguments`: Worker-local plugin owner and component operation.
    ///
    /// # Returns
    /// The upstream mailbox acknowledgement.
    ///
    /// # Errors
    /// Returns a denial for malformed input, invalid owner, or mailbox failure.
    pub fn dispatch(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, String> {
        let request: ComponentRequest = serde_json::from_value(arguments.clone())
            .map_err(|_| "plugin UI component request is invalid".to_owned())?;
        if request.owner == 0 {
            return Err("plugin UI component owner is invalid".to_owned());
        }
        let owner = {
            let mut owners = self
                .owners
                .lock()
                .map_err(|_| "plugin UI component registry is unavailable".to_owned())?;
            *owners
                .entry(request.owner)
                .or_insert_with(|| NEXT_OWNER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
        };
        self.mailbox.component(owner, request.arguments)
    }
}

impl Drop for PluginComponentDispatcher {
    /// Removes every component owner published by this worker.
    fn drop(&mut self) {
        if let Ok(owners) = self.owners.lock() {
            for owner in owners.values() {
                self.mailbox.remove_owner(*owner);
            }
        }
    }
}

/// Wire payload for a plugin component action forwarded by the worker.
///
/// # Public members
/// None.
///
/// # Private members
/// - `owner`: Plugin ID assigned inside the worker.
/// - `arguments`: Upstream component lifecycle payload.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ComponentRequest {
    /// Plugin identity assigned inside the worker process.
    owner: u64,
    /// Component registration or update operation.
    arguments: serde_json::Value,
}
