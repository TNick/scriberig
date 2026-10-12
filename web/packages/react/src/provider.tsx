/** @file Provides shared React context for Scriberig model and editor bindings. */
import { createContext, useContext, type ReactNode } from "react";
import type { ScriberigClient } from "@scriberig/client";
import type { WorkspaceModel } from "@scriberig/model";
import type { CapabilitiesResponse } from "@scriberig/protocol";
import type { CommandDescriptor, KnowledgeTree, SelectionController } from "@scriberig/editor-core";

/** Objects shared by all Scriberig React bindings. */
export interface ScriberigReactValue {
  /** Framework-independent semantic client. */
  client: ScriberigClient;
  /** Shared semantic model. */
  model: WorkspaceModel;
  /** Shared semantic selection controller. */
  selection: SelectionController;
  /** Commands discovered by the host application. */
  commands: readonly CommandDescriptor[];
  /** Optional shared knowledge tree controller. */
  tree?: KnowledgeTree;
  /** Capability discovery supplied by the host runtime, when available. */
  capabilities?: CapabilitiesResponse | undefined;
}

const ScriberigContext = createContext<ScriberigReactValue | null>(null);

/** Provides already-created semantic objects to descendant hooks.
 * @param props Provider properties.
 * @param props.value Shared runtime objects.
 * @param props.children Descendant React elements.
 * @returns A context provider element.
 * @throws Error when provider configuration is invalid.
 */
export function ScriberigProvider({
  value,
  children,
}: {
  value: ScriberigReactValue;
  children: ReactNode;
}) {
  return <ScriberigContext.Provider value={value}>{children}</ScriberigContext.Provider>;
}

/** Reads the nearest Scriberig provider value.
 * @returns Shared runtime objects.
 * @throws Error when called outside a ScriberigProvider.
 */
export function useScriberig(): ScriberigReactValue {
  const value = useContext(ScriberigContext);
  if (!value) throw new Error("useScriberig must be used inside ScriberigProvider");
  return value;
}
