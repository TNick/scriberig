/** @file Hosts registered domain projection handlers. */
import type { ReactElement } from "react";
import type { ProjectionHandler } from "./component-types";

/** Selects a handler and exposes explicit unsupported or readonly states. */
export function ProjectionHost({
  projectionId,
  handlers,
  selection,
  readonly,
  className,
}: {
  projectionId: string;
  handlers: readonly ProjectionHandler[];
  selection?: string;
  readonly?: boolean;
  className?: string;
}): ReactElement {
  const handler = handlers.find((candidate) => candidate.id === projectionId);
  if (!handler)
    return (
      <section className={className} role="status">
        Unsupported projection: {projectionId}
      </section>
    );
  const context = selection === undefined ? {} : { selection };
  return (
    <section className={className} aria-label={readonly ? "Readonly projection" : "Projection"}>
      {readonly ? <p role="status">Readonly</p> : null}
      {handler.render(context)}
    </section>
  );
}
