/** @file Defines React bindings for headless trust inspection state. */

import { useEffect, useMemo, useState } from "react";
import type { TrustClient } from "@scriberig/client";
import { TrustInspector, type TrustInspectorState } from "@scriberig/model";
import type { TrustSelection } from "@scriberig/protocol";

/** Loads trust details for the current universal semantic selection. */
/**
 * @param client Trust data client.
 * @param selection Universal semantic selection or no selection.
 * @returns Current headless inspector state.
 */
export function useTrustSummary(
  client: TrustClient,
  selection: TrustSelection | null,
): TrustInspectorState {
  const inspector = useMemo(() => new TrustInspector(), []);
  const [state, setState] = useState<TrustInspectorState>({ status: "idle" });

  useEffect(() => {
    if (!selection) {
      setState({ status: "idle" });
      return () => inspector.dispose();
    }
    let active = true;
    void inspector.load(client, selection).then((next) => {
      if (active) setState(next);
    });
    return () => {
      active = false;
      inspector.dispose();
    };
  }, [client, inspector, selection]);

  return state;
}

/** Loads detailed provenance through the same model boundary. */
export const useProvenance = useTrustSummary;
/** Loads validation state through the same model boundary. */
export const useValidation = useTrustSummary;
/** Subscribes to trust events through the same model boundary. */
export const useTrustEvents = useTrustSummary;
