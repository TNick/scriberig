/** @file Composes the injectable runtime from public web packages. */
import { HttpTransport, ScriberigClient } from "@scriberig/client";
import { SelectionController, type CommandDescriptor } from "@scriberig/editor-core";
import { KnowledgeTree, type TreeNode } from "@scriberig/editor-core";
import { WorkspaceModel } from "@scriberig/model";

/** Browser connection state reported by the application runtime. */
export type ConnectionState = "connecting" | "connected" | "disconnected";

/** Shared application runtime dependencies. */
export interface AppRuntime {
  client: ScriberigClient;
  model: WorkspaceModel;
  selection: SelectionController;
  commands: readonly CommandDescriptor[];
  tree: KnowledgeTree;
  connection: ConnectionState;
  bootstrap: () => Promise<void>;
}

/** Creates the application runtime from public package contracts. */
/**
 * @param baseUrl Base URL used by the public HTTP transport.
 * @returns Injectable runtime dependencies and bootstrap controls.
 * @throws The bootstrap promise rejects when the server is unavailable or
 *     returns an invalid capability/inspection response.
 */
export function createRuntime(baseUrl = "/api"): AppRuntime {
  const transport = new HttpTransport(baseUrl);
  const tree = new KnowledgeTree();
  let connection: ConnectionState = "connecting";
  let bootstrapPromise: Promise<void> | undefined;
  const client = new ScriberigClient(transport);
  const bootstrap = async (): Promise<void> => {
    if (bootstrapPromise) return bootstrapPromise;
    bootstrapPromise = (async () => {
      const capabilities = await client.capabilities();
      if (!capabilities.root) throw new Error("server did not advertise a knowledge root");
      const entityId = capabilities.root.entityId;
      const snapshotId = capabilities.root.snapshotId;
      const description = await client.inspect({
        entityId,
        snapshot: { kind: "exact", snapshotId },
        detail: "compact",
      });
      if (!("summary" in description)) throw new Error("server returned an invalid inspection");
      const node: TreeNode = {
        id: "root",
        entityId,
        snapshotId,
        summary: description.summary,
        expanded: false,
        loading: false,
        children: [],
        nextCursor: "",
      };
      tree.setRoot(node);
      connection = "connected";
    })().catch((error) => {
      connection = "disconnected";
      throw error;
    });
    return bootstrapPromise;
  };
  return {
    client,
    model: new WorkspaceModel(),
    selection: new SelectionController(),
    commands: [],
    tree,
    get connection() {
      return connection;
    },
    bootstrap,
  };
}
