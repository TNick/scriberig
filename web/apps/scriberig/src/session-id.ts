/** @file Creates and persists protocol-valid browser session identities. */

import { newSessionId, type SessionId } from "@scriberig/protocol";

const SESSION_STORAGE_KEY = "scriberig.agent.session";

/** Reuses a valid stored session id or creates a canonical random one.
 * @param storage Optional storage adapter, primarily for tests.
 * @returns A Rust-compatible `ent_` session identity.
 * @throws When secure random generation is unavailable.
 */
export function getOrCreateSessionId(storage?: Pick<Storage, "getItem" | "setItem">): SessionId {
  let activeStorage = storage;
  if (activeStorage === undefined) {
    try {
      activeStorage = globalThis.sessionStorage;
    } catch {
      activeStorage = undefined;
    }
  }
  if (activeStorage !== undefined) {
    try {
      const stored = activeStorage.getItem(SESSION_STORAGE_KEY);
      if (stored !== null && /^ent_[a-f0-9]{32}$/.test(stored)) return stored as SessionId;
    } catch {
      activeStorage = undefined;
    }
  }
  const sessionId = newSessionId();
  try {
    activeStorage?.setItem(SESSION_STORAGE_KEY, sessionId);
  } catch {
    // Storage may be unavailable; the current view retains the valid id in memory.
  }
  return sessionId;
}
