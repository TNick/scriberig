/** @file Renders a durable operation lifecycle. */
import type { ReactElement } from "react";

/** Displays operation progress and result references without cancelling it. */
export function OperationView({
  id,
  status,
  progress,
  result,
  className,
}: {
  id: string;
  status: string;
  progress?: number;
  result?: string;
  className?: string;
}): ReactElement {
  return (
    <section className={className} aria-label={`Operation ${id}`}>
      <p>Status: {status}</p>
      {progress !== undefined ? (
        <progress value={progress} max={100}>
          {progress}%
        </progress>
      ) : null}
      {result ? <p>Result: {result}</p> : null}
    </section>
  );
}
