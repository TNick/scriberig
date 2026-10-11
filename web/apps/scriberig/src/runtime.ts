/** @file Composes the injectable runtime from public web packages. */
import { HttpTransport, ScriberigClient } from "@scriberig/client";
import { SelectionController, type CommandDescriptor } from "@scriberig/editor-core";
import { KnowledgeTree, type TreeNode } from "@scriberig/editor-core";
import { WorkspaceModel } from "@scriberig/model";
import type { CapabilitiesResponse, ProtocolEvent } from "@scriberig/protocol";

/** Browser connection state reported by the application runtime. */
export type ConnectionState = "connecting" | "connected" | "disconnected";

/** Shared application runtime dependencies. */
export interface AppRuntime {
  client: ScriberigClient;
  model: WorkspaceModel;
  selection: SelectionController;
  commands: readonly CommandDescriptor[];
  tree: KnowledgeTree;
  capabilities: CapabilitiesResponse | undefined;
  connection: ConnectionState;
  bootstrap: () => Promise<void>;
}

const COMMANDS: Readonly<Record<string, Omit<CommandDescriptor, "requiredCapability">>> = {
  inspect: { id: "inspect", title: "Inspect selection", mutating: false, execute: "inspect" },
  list: { id: "list", title: "Browse children", mutating: false, execute: "list" },
  find: { id: "find", title: "Find objects", mutating: false, execute: "find" },
  query: { id: "query", title: "Run query", mutating: false, execute: "query" },
  history: { id: "history", title: "Open history", mutating: false, execute: "history" },
  diff: { id: "diff", title: "Compare snapshots", mutating: false, execute: "diff" },
  migrate: { id: "migrate", title: "Propose migration", mutating: true, execute: "migrate" },
  validate: { id: "validate", title: "Validate proposal", mutating: false, execute: "validate" },
  render: { id: "render", title: "Render projection", mutating: false, execute: "render" },
  session: { id: "session", title: "Open session", mutating: false, execute: "session" },
  agent: { id: "agent", title: "Run agent operation", mutating: true, execute: "agent" },
};

/** Filters the public command catalog by capabilities advertised by the server.
 *
 * @param capabilities Runtime capability discovery response.
 * @returns Commands available to the current runtime.
 * @throws This pure filter does not throw unless capability data is malformed.
 */
function commandsFor(capabilities: CapabilitiesResponse): readonly CommandDescriptor[] {
  const supported = new Set([
    ...capabilities.features,
    ...capabilities.capabilities
      .filter((capability) => capability.available)
      .map((capability) => capability.id),
  ]);
  return Object.entries(COMMANDS)
    .filter(([id]) => supported.has(id))
    .map(([id, command]) => ({ ...command, requiredCapability: id }));
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
  let discovered: CapabilitiesResponse | undefined;
  let commands: readonly CommandDescriptor[] = [];
  const client = new ScriberigClient(transport);
  const model = new WorkspaceModel();
  const bootstrap = async (): Promise<void> => {
    if (bootstrapPromise) return bootstrapPromise;
    bootstrapPromise = (async () => {
      const capabilities = await client.capabilities();
      discovered = capabilities;
      commands = commandsFor(capabilities);
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
      model.setDisconnected(false);
      client.events((event: ProtocolEvent) => model.apply(event));
      connection = "connected";
    })().catch((error) => {
      connection = "disconnected";
      model.setDisconnected(true);
      throw error;
    });
    return bootstrapPromise;
  };
  return {
    client,
    model,
    selection: new SelectionController(),
    get capabilities() {
      return discovered;
    },
    get commands() {
      return commands;
    },
    tree,
    get connection() {
      return connection;
    },
    bootstrap,
  };
}
