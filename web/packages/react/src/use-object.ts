/** @file Loads exact semantic object state through the shared client. */
import { useEffect, useState } from "react";
import type { InspectRequest, ObjectDescription, SnapshotId, EntityId } from "@scriberig/protocol";
import { useScriberig } from "./provider";

/** Object loading states preserve exact historical requests. */
export type ObjectState =
  | { status: "idle" | "loading" }
  | { status: "loaded"; value: ObjectDescription }
  | { status: "error"; error: unknown };
/** Loads one semantic object through the client boundary.
 * @param request Exact inspect request, or undefined to remain idle.
 * @returns Loading, success, or error state.
 */
export function useObject(
  request?: InspectRequest | { entityId: EntityId; snapshotId: SnapshotId },
): ObjectState {
  const { client } = useScriberig();
  const [state, setState] = useState<ObjectState>({ status: "idle" });
  useEffect(() => {
    if (!request) {
      setState({ status: "idle" });
      return;
    }
    const controller = new AbortController();
    setState({ status: "loading" });
    const inspect: InspectRequest =
      "snapshot" in request
        ? request
        : {
            entityId: request.entityId,
            snapshot: { kind: "exact", snapshotId: request.snapshotId },
            detail: "deep",
          };
    void client.inspect(inspect, controller.signal).then(
      (value) => setState({ status: "loaded", value: value as ObjectDescription }),
      (error: unknown) => {
        if (!controller.signal.aborted) setState({ status: "error", error });
      },
    );
    return () => controller.abort();
  }, [client, request]);
  return state;
}
