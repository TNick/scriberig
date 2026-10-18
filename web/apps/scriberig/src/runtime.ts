/** @file Composes the injectable runtime from public web packages. */
import { HttpTransport, ScriberigClient } from "@scriberig/client";
import { SelectionController, type CommandDescriptor } from "@scriberig/editor-core";
import { KnowledgeTree, type TreeNode } from "@scriberig/editor-core";
import { WorkspaceModel } from "@scriberig/model";
import type { AgentEvent, CapabilitiesResponse, ProtocolEvent } from "@scriberig/protocol";

const ACTIVE_PROJECT_STORAGE_KEY = "scriberig.active.project";

/** Project workspace resolved from the authoritative application service. */
export interface ActiveWorkspace {
  project: { id: string; name: string; path: string; status: string; snapshot?: string };
  snapshot: string;
  rootEntity: string;
}

/** Named semantic symbol exposed by the shared project adapter. */
export interface ProjectSymbol {
  entityId: string;
  snapshotId: string;
  path: string;
  kind: string;
  name: string;
}

/** Candidate returned by the shared human or agent review path. */
export interface ProjectCandidate {
  /** Stable candidate identity. */
  candidate: string;
  /** Exact base snapshot. */
  base: string;
  /** Changed project-relative path. */
  path: string;
  /** Original source text. */
  old: string;
  /** Proposed source text. */
  new: string;
  /** Validation terminal state. */
  validation: string;
  /** Whether normal Apply is available. */
  applyEnabled: boolean;
}

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
  activeWorkspace: ActiveWorkspace | undefined;
  symbols: ProjectSymbol[];
  agentCandidate: ProjectCandidate | undefined;
  activateProject: (projectId: string) => Promise<void>;
  setAgentCandidate: (candidate: ProjectCandidate | undefined) => void;
  subscribe: (listener: () => void) => () => void;
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

/** Checks whether a discovered capability or feature is available.
 * @param capabilities Capability discovery response.
 * @param id Capability or feature identifier.
 * @returns Whether the capability is currently available.
 * @throws This pure projection does not throw.
 */
export function capabilityAvailable(
  capabilities: CapabilitiesResponse | undefined,
  id: string,
): boolean {
  return (
    capabilities?.features.includes(id) === true ||
    capabilities?.capabilities.some(
      (capability) => capability.id === id && capability.available,
    ) === true
  );
}

/** Projects a wire operation event into the branch-aware agent model.
 * @param model Shared semantic model receiving the event.
 * @param event Normalized protocol event from the server.
 * @param fallbackSequence Local sequence used by transports without cursors.
 * @returns The next sequence value.
 * @throws This pure projection does not throw.
 */
function applyAgentProtocolEvent(
  model: WorkspaceModel,
  event: ProtocolEvent,
  fallbackSequence: number,
): number {
  const sequence = Math.max(fallbackSequence + 1, event.sequence ?? fallbackSequence + 1);
  let agentEvent: AgentEvent | undefined;
  switch (event.kind) {
    case "agentOutput":
      agentEvent = {
        kind: "agentOutputCommitted",
        sequence,
        operationId: event.operationId,
      };
      break;
    case "agentActivity":
      agentEvent = {
        kind: "transientActivity",
        sequence,
        operationId: event.operationId,
        branchId: event.branchId,
        activityKind: event.activityKind,
        state: event.state,
        summary: event.summary,
      };
      break;
    case "branchChanged":
      agentEvent = { kind: "branchChanged", sequence, branchId: event.branchId as never };
      break;
    case "sessionChanged":
      agentEvent = { kind: "branchChanged", sequence, branchId: event.sessionId as never };
      break;
    case "operationProgress":
      agentEvent = { kind: "operationProgress", sequence, operationId: event.operationId };
      break;
    case "operationCompleted":
      agentEvent = { kind: "operationCompleted", sequence, operationId: event.operationId };
      break;
    case "operationFailed":
      agentEvent = {
        kind: "operationFailed",
        sequence,
        operationId: event.operationId,
        failure: { category: "operation_failure", message: event.error },
      };
      break;
    case "operationCancelled":
      agentEvent = { kind: "operationCancelled", sequence, operationId: event.operationId };
      break;
    default:
      break;
  }
  if (agentEvent !== undefined) model.applyAgentEvent(agentEvent);
  return sequence;
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
  let agentSequence = 0;
  let agentRefresh: Promise<void> | undefined;
  let activeWorkspace: ActiveWorkspace | undefined;
  let symbols: ProjectSymbol[] = [];
  let agentCandidate: ProjectCandidate | undefined;
  const listeners = new Set<() => void>();
  const client = new ScriberigClient(transport);
  const model = new WorkspaceModel();
  const refreshAgentWorkspace = async (): Promise<void> => {
    if (agentRefresh !== undefined) return agentRefresh;
    agentRefresh = (async () => {
      const operations = model.getAgentState().operations;
      await Promise.all(
        Object.values(operations).map(async (known) => {
          const operation = await client.getOperation(known.operationId);
          model.operations.set(operation);
          model.setAgentOperation(operation);
          const sessionId = operation.sessionId ?? known.sessionId;
          if (sessionId !== undefined) {
            const session = await client.getSession(sessionId);
            model.setAgentSession(session);
          }
        }),
      );
    })().finally(() => {
      agentRefresh = undefined;
    });
    return agentRefresh;
  };
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
      client.events(
        (event: ProtocolEvent) => {
          model.apply(event);
          agentSequence = applyAgentProtocolEvent(model, event, agentSequence);
          if (
            event.kind === "operationCompleted" ||
            event.kind === "operationFailed" ||
            event.kind === "operationCancelled"
          ) {
            void client.operation(event.operationId).then((operation) => {
              model.operations.set(operation);
              model.setAgentOperation(operation);
              if (operation.sessionId !== undefined) {
                void client
                  .getSession(operation.sessionId)
                  .then((session) => model.setAgentSession(session));
              }
            });
          }
        },
        undefined,
        (state) => {
          const disconnected = state === "disconnected";
          connection = disconnected ? "disconnected" : "connected";
          model.setDisconnected(disconnected);
          if (disconnected) {
            model.markAgentWorkspaceStale();
          } else {
            void refreshAgentWorkspace();
          }
        },
      );
      connection = "connected";
      const storedProject = globalThis.localStorage?.getItem(ACTIVE_PROJECT_STORAGE_KEY);
      if (storedProject !== null && storedProject !== undefined) {
        await activateProject(storedProject);
      }
    })().catch((error) => {
      connection = "disconnected";
      model.setDisconnected(true);
      throw error;
    });
    return bootstrapPromise;
  };
  const activateProject = async (projectId: string): Promise<void> => {
    const workspace = await client.project<ActiveWorkspace>("workspace", { project: projectId });
    const value = await client.project<{ symbols: ProjectSymbol[] }>("symbols", {
      project: projectId,
    });
    activeWorkspace = workspace;
    symbols = value.symbols;
    globalThis.localStorage?.setItem(ACTIVE_PROJECT_STORAGE_KEY, projectId);
    const rootDescription = await client.inspect({
      entityId: workspace.rootEntity as never,
      snapshot: { kind: "exact", snapshotId: workspace.snapshot as never },
      detail: "compact",
    });
    if ("summary" in rootDescription) {
      tree.setRoot({
        id: "root",
        entityId: workspace.rootEntity as never,
        snapshotId: workspace.snapshot as never,
        summary: rootDescription.summary,
        expanded: false,
        loading: false,
        children: [],
      });
    }
    listeners.forEach((listener) => listener());
  };
  const setAgentCandidate = (candidate: ProjectCandidate | undefined): void => {
    agentCandidate = candidate;
    listeners.forEach((listener) => listener());
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
    get activeWorkspace() {
      return activeWorkspace;
    },
    get symbols() {
      return symbols;
    },
    get agentCandidate() {
      return agentCandidate;
    },
    activateProject,
    setAgentCandidate,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}
