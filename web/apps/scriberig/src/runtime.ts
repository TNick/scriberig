/** @file Composes the injectable runtime from public web packages. */
import { HttpTransport, ScriberigClient } from "@scriberig/client";
import { SelectionController, type CommandDescriptor } from "@scriberig/editor-core";
import { WorkspaceModel } from "@scriberig/model";

/** Shared application runtime dependencies. */
export interface AppRuntime {
  client: ScriberigClient;
  model: WorkspaceModel;
  selection: SelectionController;
  commands: readonly CommandDescriptor[];
}

/** Creates the application runtime from public package contracts. */
/** @returns The rendered createRuntime view or runtime value. */
export function createRuntime(baseUrl = "/api"): AppRuntime {
  const transport = new HttpTransport(baseUrl);
  return {
    client: new ScriberigClient(transport),
    model: new WorkspaceModel(),
    selection: new SelectionController(),
    commands: [],
  };
}
