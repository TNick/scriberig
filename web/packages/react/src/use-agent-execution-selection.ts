/** @file Binds shared session model selection to the semantic client and model. */

import { useCallback, useMemo, useState } from "react";
import type { ScriberigClient } from "@scriberig/client";
import type { WorkspaceModel } from "@scriberig/model";
import type { SessionId } from "@scriberig/protocol";
import { useScriberig } from "./provider";

/** Operations for changing the execution selection of one session.
 * @param client Shared semantic client.
 * @param model Shared client model.
 * @param sessionId Session whose future invocations use the selection.
 * @returns Model and thinking selection actions.
 * @throws When a selection mutation fails after refreshing the session.
 */
export function createAgentExecutionSelectionController(
  client: Pick<ScriberigClient, "getSession" | "setSessionModel" | "setSessionThinkingLevel">,
  model: Pick<WorkspaceModel, "getAgentState" | "setAgentSession">,
  sessionId: SessionId,
): {
  selectModel: (providerId: string, modelId: string) => Promise<void>;
  selectThinkingLevel: (levelId: string) => Promise<void>;
} {
  const refresh = async (): Promise<void> => {
    try {
      model.setAgentSession(await client.getSession(sessionId));
    } catch {
      // Keep the last authoritative session snapshot when refresh is unavailable.
    }
  };
  const selectModel = async (providerId: string, modelId: string): Promise<void> => {
    const revision = model.getAgentState().execution.revision;
    try {
      await client.setSessionModel(sessionId, revision, providerId, modelId);
      await refresh();
    } catch (error) {
      await refresh();
      throw error;
    }
  };
  const selectThinkingLevel = async (levelId: string): Promise<void> => {
    const revision = model.getAgentState().execution.revision;
    try {
      await client.setSessionThinkingLevel(sessionId, revision, levelId);
      await refresh();
    } catch (error) {
      await refresh();
      throw error;
    }
  };
  return { selectModel, selectThinkingLevel };
}

/** Exposes shared model/thinking mutations with loading state to React views.
 * @param sessionId Session whose future invocations use the selection.
 * @returns Selection actions and whether one is in flight.
 */
export function useAgentExecutionSelection(sessionId: SessionId): {
  selectModel: (providerId: string, modelId: string) => Promise<void>;
  selectThinkingLevel: (levelId: string) => Promise<void>;
  mutating: boolean;
} {
  const { client, model } = useScriberig();
  const [mutating, setMutating] = useState(false);
  const controller = useMemo(
    () => createAgentExecutionSelectionController(client, model, sessionId),
    [client, model, sessionId],
  );
  const selectModel = useCallback(
    async (providerId: string, modelId: string): Promise<void> => {
      setMutating(true);
      try {
        await controller.selectModel(providerId, modelId);
      } finally {
        setMutating(false);
      }
    },
    [controller],
  );
  const selectThinkingLevel = useCallback(
    async (levelId: string): Promise<void> => {
      setMutating(true);
      try {
        await controller.selectThinkingLevel(levelId);
      } finally {
        setMutating(false);
      }
    },
    [controller],
  );
  return { selectModel, selectThinkingLevel, mutating };
}
