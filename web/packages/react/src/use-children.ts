/** @file Loads one snapshot-aware relation page for React consumers. */
import { useEffect, useState } from "react";
import type { CollectionPage, ListRequest } from "@scriberig/protocol";
import { useScriberig } from "./provider";
/** Children loading state. */
export type ChildrenState =
  | { status: "idle" | "loading" }
  | { status: "loaded"; page: CollectionPage }
  | { status: "error"; error: unknown };
/** Loads one non-recursive child page.
 * @param request Exact relation request, or undefined.
 * @returns Child page state.
 */
export function useChildren(request?: ListRequest): ChildrenState {
  const { client } = useScriberig();
  const [state, setState] = useState<ChildrenState>({ status: "idle" });
  useEffect(() => {
    if (!request) {
      setState({ status: "idle" });
      return;
    }
    const controller = new AbortController();
    setState({ status: "loading" });
    void client.list(request, controller.signal).then(
      (value) => setState({ status: "loaded", page: value as CollectionPage }),
      (error: unknown) => {
        if (!controller.signal.aborted) setState({ status: "error", error });
      },
    );
    return () => controller.abort();
  }, [client, request]);
  return state;
}
