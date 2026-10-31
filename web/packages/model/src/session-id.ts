/** @file Creates secure session identifiers for browser-side semantic state. */
import type { SessionId } from "@scriberig/protocol";

/** Creates a random session identifier in the canonical Rust entity format.
 * @returns A 128-bit random `ent_` identifier branded as a session id.
 * @throws When the runtime does not provide a cryptographically secure random source.
 */
export function newSessionId(): SessionId {
  const cryptoApi = globalThis.crypto;
  if (cryptoApi === undefined || typeof cryptoApi.getRandomValues !== "function") {
    throw new Error("secure session identifiers require Web Crypto");
  }
  const bytes = cryptoApi.getRandomValues(new Uint8Array(16));
  const suffix = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  return `ent_${suffix}` as SessionId;
}
