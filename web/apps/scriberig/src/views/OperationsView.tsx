/** @file Provides independently inspectable operation state. */
import { MuiOperationView } from "@scriberig/components-mui";
import { useScriberig } from "@scriberig/react";
import { useSyncExternalStore, type ReactElement } from "react";
/** Renders durable operation status independently of initiating views. */
/** @returns The rendered OperationsView view or runtime value. */
export function OperationsView(): ReactElement {
  const { model } = useScriberig();
  const operations = useSyncExternalStore(
    model.operations.subscribe.bind(model.operations),
    () => model.operations.all(),
    () => model.operations.all(),
  );
  return (
    <section aria-label="Operations">
      <h1>Operations</h1>
      {operations.length === 0 ? <p role="status">No active operations.</p> : null}
      {operations.map((operation) => (
        <MuiOperationView
          key={operation.operationId}
          id={operation.operationId}
          status={operation.status}
          {...(operation.progress === undefined ? {} : { progress: operation.progress * 100 })}
          {...(operation.resultSnapshotId === undefined
            ? {}
            : { result: operation.resultSnapshotId })}
        />
      ))}
    </section>
  );
}
