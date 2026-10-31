/** @file Defines browser-safe contextual action contracts for page helpers. */

/** Describes one operation that a page exposes to a contextual helper. */
export interface ContextualActionDescriptor {
  /** Stable action identifier. */
  id: string;
  /** Human-readable action label. */
  label: string;
  /** Whether the action can change durable application state. */
  mutating: boolean;
  /** Exact page context revision used when the action was described. */
  contextVersion: number;
}

/** Identifies one browser action invocation and its page context. */
export interface ContextualActionRequest {
  /** Unique invocation identifier. */
  invocationId: string;
  /** Action selected by the helper. */
  actionId: string;
  /** Context revision the helper observed. */
  contextVersion: number;
}

/** Reports the result of a contextual action invocation. */
export interface ContextualActionResult {
  /** Invocation identifier copied from the request. */
  invocationId: string;
  /** Whether the action completed successfully. */
  ok: boolean;
  /** Safe user-facing summary. */
  summary: string;
  /** New context revision after the action, when the page changed. */
  contextVersion: number;
}

/** Executes a validated browser action request. */
export type ContextualActionExecutor = (
  request: ContextualActionRequest,
) => Promise<ContextualActionResult>;

/** Owns the current page action catalog and validates stale invocations.
 *
 * Public members: `set`, `list`, and `validate`.
 * Private members: the in-memory action map.
 */
export class ContextualActionRegistry {
  private actions = new Map<string, ContextualActionDescriptor>();

  /** Replaces the page action catalog with a new immutable context snapshot.
   * @param actions Actions available in the current page context.
   * @returns Nothing.
   */
  public set(actions: readonly ContextualActionDescriptor[]): void {
    this.actions = new Map(actions.map((action) => [action.id, action]));
  }

  /** Returns the current action catalog for prompt/tool projection.
   * @returns A stable action descriptor list.
   */
  public list(): readonly ContextualActionDescriptor[] {
    return [...this.actions.values()];
  }

  /** Validates that an invocation still targets the current page context.
   * @param request Browser action request received from a helper bridge.
   * @returns A failure result for stale/unknown actions, otherwise undefined.
   */
  public validate(request: ContextualActionRequest): ContextualActionResult | undefined {
    const action = this.actions.get(request.actionId);
    if (action === undefined) {
      return {
        invocationId: request.invocationId,
        ok: false,
        summary: `Unknown page action: ${request.actionId}`,
        contextVersion: request.contextVersion,
      };
    }
    if (action.contextVersion !== request.contextVersion) {
      return {
        invocationId: request.invocationId,
        ok: false,
        summary: "The page changed; refresh context before invoking this action.",
        contextVersion: action.contextVersion,
      };
    }
    return undefined;
  }
}
