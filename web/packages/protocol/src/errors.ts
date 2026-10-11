/** @file Defines structured protocol errors for explicit UI handling. */
/** Machine-readable protocol error categories. */
export type ProtocolErrorCode =
  | "authorization_denied"
  | "not_found"
  | "stale_base"
  | "migration_conflict"
  | "reconciliation_ambiguity"
  | "validation_failure"
  | "operation_failure"
  | "unsupported_capability"
  | "protocol_incompatible";
/** Structured error returned by a protocol operation. */
export interface ProtocolError {
  code: ProtocolErrorCode;
  message: string;
  details?: Record<string, unknown>;
  correlationId?: string;
}
